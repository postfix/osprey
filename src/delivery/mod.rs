//! Where a decision record goes once this process has made it.
//!
//! `http::logging` owns "a line reaches stdout". This module owns "a copy of that
//! record leaves this process", which is not the same promise and not the same
//! failure mode: stdout cannot fill up, and a file or a collector can.
//!
//! The shape of the interface is the product promise made structural. [`Sinks::offer`]
//! is not `async`, takes no lock, does no I/O and returns nothing, so delivery cannot
//! slow or fail a request without changing a signature. A record that cannot be
//! handed over is counted rather than discarded silently, and the counts are read
//! back out by the summary path.

mod file;
mod siem;

use std::net::IpAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::redirect;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::StartupError;
use crate::config::Config;

/// How many records may wait for a sink before one is dropped. Gate 2's value: deep
/// enough to absorb a stalled write, shallow enough that a wedged sink cannot grow
/// without bound.
const QUEUE_CAPACITY: usize = 4096;

/// One line of delivered output. The tag is what lets a reader of the file tell a
/// decision from a summary without guessing at the key set.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum Record {
    RequestDecided(Decision),
    RequestSummary(Summary),
}

/// The stdout decision line's twelve SPEC §11 fields, in the order it carries them,
/// so one NDJSON line and one stdout line cannot drift apart — preceded by
/// `timestamp`, which only the file and the SIEM receive because the console
/// formatter stamps stdout itself, plus `consumer` last when an operator has opted in
/// to it.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Decision {
    pub timestamp: String,
    pub request_id: String,
    pub method: String,
    pub ecosystem: &'static str,
    pub package: String,
    pub version: String,
    pub status: u16,
    pub result: &'static str,
    pub reason: String,
    pub blocklist_revision: u64,
    pub cache: &'static str,
    pub duration_micros: u64,
    pub bytes: u64,
    /// The peer address of the connection that asked, and never its port — `Some`
    /// only while consumer identification is on. An absent field rather than a null
    /// one, so the default key set an operator reads is the twelve above.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer: Option<IpAddr>,
}

/// The periodic counter summary, plus the per-sink drop counts for the window it
/// closes — so "a record was dropped" reaches the durable destinations rather than
/// only the console.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Summary {
    pub timestamp: String,
    pub requests: u64,
    pub errors: u64,
    pub bytes: u64,
    pub mean_duration_micros: u64,
    pub window_micros: u64,
    pub dropped_file: u64,
    pub dropped_siem: u64,
}

/// `utc_micros` as UTC RFC 3339 with exactly six fractional digits, so every record's
/// timestamp has the same width. Out of `jiff`'s range, the raw count is written
/// rather than a made-up date.
pub(crate) fn rfc3339(utc_micros: i64) -> String {
    jiff::Timestamp::from_microsecond(utc_micros)
        .map(|t| format!("{t:.6}"))
        .unwrap_or_else(|_| utc_micros.to_string())
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Drops {
    pub file: u64,
    pub siem: u64,
}

/// One destination: the sending half of its queue and the counter both sides
/// increment.
struct Sink {
    tx: mpsc::Sender<Record>,
    drops: Arc<AtomicU64>,
}

impl Sink {
    fn push(&self, record: Record) {
        if self.tx.try_send(record).is_err() {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// The enabled destinations. Empty unless an operator configured one, which is what
/// makes "not on by default" a property of the value rather than of a code path.
pub(crate) struct Sinks {
    file: Option<Sink>,
    siem: Option<Sink>,
}

impl Sinks {
    /// Hands `record` to every enabled sink. Never blocks, never fails, never tells
    /// the caller anything — a full or closed queue counts a drop and returns.
    ///
    /// The record is cloned only when both sinks are enabled, so the common
    /// single-sink case moves it rather than copying it.
    pub(crate) fn offer(&self, record: Record) {
        match (&self.file, &self.siem) {
            (Some(file), Some(siem)) => {
                file.push(record.clone());
                siem.push(record);
            }
            (Some(file), None) => file.push(record),
            (None, Some(siem)) => siem.push(record),
            (None, None) => {}
        }
    }

    /// The drops accumulated since the previous call, per sink. Read-and-reset, so
    /// the counts are per summary window exactly as `requests` and `errors` are.
    ///
    /// Called only from the summary path, which serialises itself under the existing
    /// counters mutex; a second caller would silently steal a window's counts.
    pub(crate) fn drops(&self) -> Drops {
        Drops {
            file: self
                .file
                .as_ref()
                .map_or(0, |sink| sink.drops.swap(0, Ordering::Relaxed)),
            siem: self
                .siem
                .as_ref()
                .map_or(0, |sink| sink.drops.swap(0, Ordering::Relaxed)),
        }
    }

    /// True when no sink is enabled, and therefore when nothing was opened and
    /// nothing was spawned.
    ///
    /// Its one caller is `App::delivery_is_empty`, compiled only under
    /// `test-support`: the promise it reads is "an operator who configures nothing
    /// gets nothing", which product code has no reason to ask about and a witness
    /// has every reason to.
    #[cfg_attr(not(feature = "test-support"), allow(dead_code))]
    pub(crate) fn is_empty(&self) -> bool {
        self.file.is_none() && self.siem.is_none()
    }
}

/// The environment variable the SIEM credential is read from. It is read exactly once,
/// in [`build`], and is never placed on `Config` — which derives `Debug`, so any
/// `{config:?}` anywhere in the process would print every field it holds.
const SIEM_AUTH_ENV: &str = "OSPREY_SIEM_AUTH";

/// The whole of what an operator is told when that variable cannot be used. A
/// `&'static str` rather than a `String`, so no part of the rejected value can reach
/// `check_config`'s printed `path: err` (`src/main.rs`) however this is later edited.
const SIEM_AUTH_REJECTED: &str = "OSPREY_SIEM_AUTH is not a valid HTTP header value; \
     it must be printable ASCII with no line break";

/// How long one delivery attempt may take, and how long its connect may take.
/// reqwest's async client has no default timeout at all, so an untimed `POST` to a
/// collector that accepts the connection and then goes silent would outlive any drain
/// deadline.
const SIEM_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const SIEM_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

/// Builds the enabled sinks and spawns their tasks.
///
/// With neither `log_file_path` nor `siem_url` set this opens no file, constructs no
/// HTTP client, spawns nothing and returns an empty `Sinks` — the default path an
/// operator who configures nothing is on.
///
/// `drain` must be the token cancelled *after* the HTTP server has joined, never the
/// shutdown token: a sink that stopped when the server did would lose the records of
/// the requests that were still being answered.
pub(crate) fn build(
    config: &Config,
    drain: CancellationToken,
) -> Result<(Sinks, Vec<JoinHandle<()>>), StartupError> {
    let mut tasks = Vec::new();

    if let Some(path) = &config.log_file_path {
        probe(path, config.log_consumer_identification)?;
    }

    let file = config.log_file_path.as_ref().map(|path| {
        let (tx, rx) = mpsc::channel(QUEUE_CAPACITY);
        let drops = Arc::new(AtomicU64::new(0));
        tasks.push(tokio::spawn(file::run(
            path.clone(),
            config.log_file_max_bytes,
            rx,
            drain.clone(),
            Arc::clone(&drops),
        )));
        Sink { tx, drops }
    });

    // The header *name* of the credential, and only when one was actually configured.
    // The value never leaves this function except as a sensitive `HeaderValue`.
    let mut siem_auth_header = None;

    let siem = match &config.siem_url {
        Some(url) => {
            let auth = siem_auth(&config.siem_auth_header)?;
            siem_auth_header = auth.as_ref().map(|(name, _)| name.as_str().to_owned());

            let client = reqwest::Client::builder()
                // Gate 2 C4: a collector that redirects must not be able to point
                // this process — credential attached — at a host nobody configured.
                .redirect(redirect::Policy::none())
                .timeout(SIEM_REQUEST_TIMEOUT)
                .connect_timeout(SIEM_CONNECT_TIMEOUT)
                // Ignore `HTTP_PROXY`/`HTTPS_PROXY`, as the upstream registry client
                // does, so both leave the host by the same route.
                .no_proxy()
                .build()
                .map_err(|err| {
                    tracing::error!(error = %err, "the SIEM delivery client could not be built");
                    StartupError::Delivery("the SIEM delivery client could not be built")
                })?;

            let (tx, rx) = mpsc::channel(QUEUE_CAPACITY);
            let drops = Arc::new(AtomicU64::new(0));
            tasks.push(tokio::spawn(siem::run(
                client,
                url.clone(),
                auth,
                rx,
                drain.clone(),
                Arc::clone(&drops),
            )));
            Some(Sink { tx, drops })
        }
        None => None,
    };

    if config.log_file_path.is_some() || config.siem_url.is_some() {
        tracing::info!(
            log_file = ?config.log_file_path,
            log_file_max_bytes = config.log_file_max_bytes.get(),
            siem_url = config.siem_url.as_ref().map(Url::as_str),
            siem_auth_header = siem_auth_header.as_deref(),
            "decision records are delivered off this process"
        );
    }

    Ok((Sinks { file, siem }, tasks))
}

/// What an operator is told when the log file cannot be opened while peer addresses
/// would be recorded. Fixed text, so no address and no path can reach it.
const LOG_FILE_REJECTED: &str = "log_file_path cannot be opened for append, and \
     log_consumer_identification is on: peer addresses would be collected with nowhere \
     durable to go";

/// Opens the log file once at startup and lets it go: the one place a typo in
/// `log_file_path` can be told apart from working delivery, and the open that creates
/// the file `0o600`. It reads no length — `file::run` restores its own tally.
///
/// Fatal only while `consumer_identification` is on, because that is when an
/// unopenable file means peer addresses kept for nothing. With it off, a logging typo
/// is not worth an outage: one error, and the sink retries per record as before.
///
/// `std::fs` rather than `tokio::fs`: this runs once, before anything is served.
fn probe(path: &Path, consumer_identification: bool) -> Result<(), StartupError> {
    let Err(err) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
    else {
        return Ok(());
    };
    if consumer_identification {
        return Err(StartupError::Delivery(LOG_FILE_REJECTED));
    }
    tracing::error!(
        key = "log_file_path",
        path = %path.display(),
        error = %err,
        "log_file_path cannot be opened for append; decision records will be dropped and \
         counted until it can"
    );
    Ok(())
}

/// Reads the SIEM credential out of the environment, once.
///
/// Set-but-unusable is deliberately not the same as unset: shipping decision records
/// unauthenticated because the credential could not be decoded is a silent downgrade,
/// so it fails startup instead.
fn siem_auth(name: &HeaderName) -> Result<Option<(HeaderName, HeaderValue)>, StartupError> {
    let value = match std::env::var(SIEM_AUTH_ENV) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(StartupError::Delivery(SIEM_AUTH_REJECTED));
        }
    };

    let mut value =
        HeaderValue::from_str(&value).map_err(|_| StartupError::Delivery(SIEM_AUTH_REJECTED))?;
    // Redacts it in any `Debug` rendering, including reqwest's own.
    value.set_sensitive(true);
    Ok(Some((name.clone(), value)))
}
