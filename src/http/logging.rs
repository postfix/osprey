//! Per-request identity, the one decision line SPEC §11 asks for, and the periodic
//! counter summary.
//!
//! SPEC §11: "Log request ID, ecosystem, package/version when known, policy result,
//! reason, blocklist revision, cache status, duration, and bytes served. Never log
//! credentials. Emit counts and timing summaries periodically to stdout."
//!
//! The request ID is a task-local rather than a handler argument, for one reason: the
//! client-visible error body is built inside [`ApiError::into_response`], which is
//! handed a `self` and nothing else. A task-local is readable from there, so the ID a
//! client is told and the ID the decision line carries are the same string by
//! construction rather than by two call sites agreeing.
//!
//! No client-supplied header, body or connection value reaches a log line. The fields
//! below come from the request target and from this process's own decision; headers,
//! bodies and upstream URLs do not appear, so a credential cannot arrive in one. The
//! request target is itself client-supplied: `package` and `version` have always been
//! derived from the request path, bounded by [`Target::of`].
//!
//! `consumer` is the one value taken from the connection, and only when an operator
//! sets `log_consumer_identification`: the peer address of the accepted TCP
//! connection, which this process observes rather than the caller claims — never a
//! forwarding header, never the port.
//!
//! Two retained analyzer specs are tripwires for this, not proofs of it:
//! `.smtc/analyzers/client-data-reaches-delivery-sink.yaml` passes at zero
//! `headers`/`body`/`from_request`-style reads of an inbound request, and
//! `.smtc/analyzers/consumer-identity-extension-read.yaml` passes at exactly one
//! inbound `extensions()` read — the `ConnectInfo` read in [`decide`].

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::body::HttpBody;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;

use crate::App;
use crate::clock::Clock;
use crate::delivery::{self, Decision, Record, Sinks, Summary};
use crate::http::error::ApiError;
use crate::policy::Ecosystem;

/// How much of a request target is allowed into a log line. npm caps a package name
/// at 214 characters and a filename is bounded by the same document, so anything
/// longer is a caller trying to write the log rather than fetch a package.
const MAX_LOGGED_TARGET: usize = 256;

/// How often the counter summary is emitted, in milliseconds.
static SUMMARY_MILLIS: AtomicU64 = AtomicU64::new(60_000);

tokio::task_local! {
    /// Set once per request by [`decide`], and read by anything that builds part of
    /// that request's answer.
    static CONTEXT: Arc<RequestContext>;
}

/// Whether local state answered the request, for the `cache` field SPEC §11 names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheStatus {
    /// The route keeps no cache, or the request failed before one was consulted.
    Unattempted,
    /// Answered from memory or from verified bytes already on disk.
    Hit,
    /// Upstream was reached for it.
    Miss,
}

impl CacheStatus {
    fn as_str(self) -> &'static str {
        match self {
            CacheStatus::Unattempted => "none",
            CacheStatus::Hit => "hit",
            CacheStatus::Miss => "miss",
        }
    }

    fn from_code(code: u8) -> CacheStatus {
        match code {
            1 => CacheStatus::Hit,
            2 => CacheStatus::Miss,
            _ => CacheStatus::Unattempted,
        }
    }

    fn code(self) -> u8 {
        match self {
            CacheStatus::Unattempted => 0,
            CacheStatus::Hit => 1,
            CacheStatus::Miss => 2,
        }
    }
}

/// What one request accumulates while it is being answered.
struct RequestContext {
    id: String,
    cache: AtomicU8,
    /// The ecosystem of the record this request turned out to be about, once a
    /// handler has read it out of the store. Zero until then, and the request target
    /// answers for it.
    ecosystem: AtomicU8,
}

/// The codes [`RequestContext::ecosystem`] holds. Zero is "nothing recorded".
const ECOSYSTEM_NPM: u8 = 1;
const ECOSYSTEM_PYPI: u8 = 2;

/// The request ID of the request being answered on this task.
///
/// Outside a request — a background task that logs an error, say — there is no
/// request to identify, and a fresh identifier is returned rather than a shared
/// placeholder so two unrelated lines never look like one request.
pub fn request_id() -> String {
    CONTEXT
        .try_with(|context| context.id.clone())
        .unwrap_or_else(|_| next_id())
}

/// Records how this request was answered from local state. Called by the code that
/// knows — the metadata and artifact paths — and ignored outside a request.
pub fn record_cache(status: CacheStatus) {
    let _ = CONTEXT.try_with(|context| context.cache.store(status.code(), Ordering::Relaxed));
}

/// Records the ecosystem of the record this request is about, read from the store
/// rather than from the path.
///
/// The artifact routes need this: a reference id is addressed under an ecosystem root
/// the caller chose, and the decision line has to name the ecosystem whose policy
/// decided the request even when the two disagree and the request is refused for it.
pub fn record_ecosystem(ecosystem: Ecosystem) {
    let code = match ecosystem {
        Ecosystem::Npm => ECOSYSTEM_NPM,
        Ecosystem::PyPi => ECOSYSTEM_PYPI,
    };
    let _ = CONTEXT.try_with(|context| context.ecosystem.store(code, Ordering::Relaxed));
}

/// The middleware that gives a request its identity and writes its decision line.
///
/// It is the outermost layer, so the line it writes describes the response that
/// actually left the process, including one produced by the router itself — an
/// unknown route or an unsupported method never reaches a handler and would
/// otherwise be the one request in the table with no decision line.
pub async fn decide(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let started = Instant::now();
    let target = Target::of(request.uri().path());
    let method = request.method().clone();

    // Read here and nowhere else: `next.run(request)` below consumes the request, so
    // afterwards there is no request left to ask. The extension exists only when the
    // server was built with connect-info, which happens only when the opt-in is on;
    // the port is deliberately dropped, because the peer address is the whole of the
    // consumer identity this product will ever record.
    let consumer = app
        .config
        .log_consumer_identification
        .then(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(peer)| peer.ip())
        })
        .flatten();

    let context = Arc::new(RequestContext {
        id: next_id(),
        cache: AtomicU8::new(CacheStatus::Unattempted.code()),
        ecosystem: AtomicU8::new(0),
    });
    let response = CONTEXT
        .scope(Arc::clone(&context), next.run(request))
        .await;

    let status = response.status();
    // A streamed artifact body has no exact size hint, but it does declare its length,
    // and that is the case where "bytes served" matters most.
    let bytes = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .or_else(|| response.body().size_hint().exact())
        .unwrap_or(0);
    let elapsed = started.elapsed();
    let error = response.extensions().get::<ApiError>().copied();
    let (result, reason) = match error {
        Some(error) => (error.error_code(), error.reason()),
        None => ("ALLOWED", "the request was served".to_owned()),
    };

    // A handler that read a record knows better than the path does.
    let ecosystem = match context.ecosystem.load(Ordering::Relaxed) {
        ECOSYSTEM_NPM => Ecosystem::Npm.as_tag(),
        ECOSYSTEM_PYPI => Ecosystem::PyPi.as_tag(),
        _ => target.ecosystem,
    };

    // The record is built first and the line is rendered from it, so the fields an
    // operator sees on stdout and the fields a sink delivers are the same fields by
    // construction rather than by two call sites agreeing.
    let decision = Decision {
        timestamp: delivery::rfc3339(app.clock.now_utc_micros()),
        request_id: context.id.clone(),
        method: method.to_string(),
        ecosystem,
        package: target.package.unwrap_or_default(),
        version: target.version.unwrap_or_default(),
        status: status.as_u16(),
        result,
        reason,
        blocklist_revision: app.blocklist_revision().unwrap_or(0),
        cache: CacheStatus::from_code(context.cache.load(Ordering::Relaxed)).as_str(),
        duration_micros: elapsed.as_micros() as u64,
        bytes,
        consumer,
    };

    tracing::info!(
        request_id = %decision.request_id,
        method = %decision.method,
        ecosystem = decision.ecosystem,
        package = decision.package.as_str(),
        version = decision.version.as_str(),
        status = decision.status,
        result = decision.result,
        reason = %decision.reason,
        blocklist_revision = decision.blocklist_revision,
        cache = decision.cache,
        duration_micros = decision.duration_micros,
        bytes = decision.bytes,
        // `Option` records nothing at all when it is `None`, so the line an operator
        // who opted out reads keeps exactly the twelve keys it has always had.
        consumer = decision.consumer.map(tracing::field::display),
        "request decided"
    );

    app.delivery.offer(Record::RequestDecided(decision));

    summarise(
        elapsed,
        bytes,
        error.is_some(),
        &app.delivery,
        app.clock.as_ref(),
    );
    response
}

/// What the request target says about itself.
///
/// Only the components of a route this server actually serves are read, and each is
/// escaped and bounded before it is logged. An unrecognised target contributes
/// nothing: a caller that invents a path does not get to choose what this process
/// writes to its own log.
struct Target {
    ecosystem: &'static str,
    package: Option<String>,
    version: Option<String>,
}

impl Target {
    fn of(path: &str) -> Target {
        let mut segments = path.split('/').skip(1);
        let root = match segments.next() {
            Some("npm") => "npm",
            Some("pypi") => "pypi",
            Some("health") => {
                return Target {
                    ecosystem: "health",
                    package: None,
                    version: None,
                };
            }
            _ => {
                return Target {
                    ecosystem: "",
                    package: None,
                    version: None,
                };
            }
        };

        let first = segments.next();
        let second = segments.next();
        let third = segments.next();
        let fourth = segments.next();
        match (root, first, second, third, fourth) {
            // `/{ecosystem}/artifacts/{reference_id}/{filename}`. Four segments, so a
            // package or project literally named `artifacts` — two segments, or three
            // with a version — reads as itself. SPEC §9: a reference id is a lookup
            // key derived from public metadata and explicitly not an authorization
            // token, so naming it is what makes one artifact request traceable to the
            // reference it asked for.
            (_, Some("artifacts"), Some(id), Some(filename), None) => Target {
                ecosystem: root,
                package: loggable(Some(id)),
                version: loggable(Some(filename)),
            },
            // Every PyPI project lives under the static `simple` segment.
            ("pypi", _, project, ..) => Target {
                ecosystem: root,
                package: loggable(project),
                version: None,
            },
            (_, package, version, ..) => Target {
                ecosystem: root,
                package: loggable(package),
                version: loggable(version),
            },
        }
    }
}

/// One route component, bounded and escaped. `Debug` on a `String` escapes control
/// characters and quotes, so a name carrying a newline cannot forge a second line.
fn loggable(segment: Option<&str>) -> Option<String> {
    let segment = segment.filter(|segment| !segment.is_empty())?;
    let bounded: String = segment.chars().take(MAX_LOGGED_TARGET).collect();
    Some(format!("{bounded:?}"))
}

fn next_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("req-{:016x}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// SPEC §11: "Emit counts and timing summaries periodically to stdout; no separate
/// metrics service is required for the MVP."
///
/// The window is closed by the request that notices it has run out, so there is no
/// background task to start, stop or leak. A window with no request in it has nothing
/// to summarise, which is exactly when nothing is written.
struct Counters {
    requests: u64,
    errors: u64,
    bytes: u64,
    micros: u64,
    opened: Instant,
}

static COUNTERS: LazyLock<Mutex<Counters>> = LazyLock::new(|| {
    Mutex::new(Counters {
        requests: 0,
        errors: 0,
        bytes: 0,
        micros: 0,
        opened: Instant::now(),
    })
});

fn summarise(elapsed: Duration, bytes: u64, was_error: bool, sinks: &Sinks, clock: &dyn Clock) {
    let window = Duration::from_millis(SUMMARY_MILLIS.load(Ordering::Relaxed));

    // Poisoning cannot lose a window: the guarded value is five integers and an
    // `Instant`, and no code between the two below can panic, so recovering from
    // a poisoned lock recovers a consistent count.
    let mut counters = COUNTERS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    counters.requests += 1;
    counters.errors += u64::from(was_error);
    counters.bytes += bytes;
    counters.micros += elapsed.as_micros() as u64;

    let open_for = counters.opened.elapsed();
    if open_for < window {
        return;
    }

    let summary = close_window(&mut counters, open_for, sinks, clock);
    drop(counters);
    emit(summary, sinks);
}

/// Closes the current window regardless of how much of it has elapsed.
///
/// Called once from `Running::shutdown`, after the HTTP server has joined — so no
/// further records can be produced — and before the drain token is cancelled, so the
/// summary is already queued when the sinks begin draining.
pub(crate) fn flush_summary(sinks: &Sinks, clock: &dyn Clock) {
    let mut counters = COUNTERS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let open_for = counters.opened.elapsed();
    let summary = close_window(&mut counters, open_for, sinks, clock);
    drop(counters);
    emit(summary, sinks);
}

/// Reports what was lost during the drain itself.
///
/// [`flush_summary`] reads and resets the drop counters *before* the drain starts, so
/// a record dropped inside the drain window would otherwise increment a counter
/// nobody ever reads again. By the time this runs the sinks have returned, so this
/// last line reaches the console only — stated rather than hidden. A clean shutdown
/// writes nothing.
pub(crate) fn flush_drop_tail(sinks: &Sinks) {
    let drops = sinks.drops();
    if drops.file != 0 || drops.siem != 0 {
        tracing::warn!(
            dropped_file = drops.file,
            dropped_siem = drops.siem,
            "decision records were dropped while delivery was draining and could not be delivered"
        );
    }
}

/// Reads the window out of `counters` and resets it. The drop counts are read inside
/// the same critical section, so one window's counts cannot be split across two.
///
/// The one place a summary reads the time, so the periodic and the shutdown summary
/// are stamped the same way: when the window is closed.
fn close_window(
    counters: &mut Counters,
    open_for: Duration,
    sinks: &Sinks,
    clock: &dyn Clock,
) -> Summary {
    let drops = sinks.drops();
    let summary = Summary {
        timestamp: delivery::rfc3339(clock.now_utc_micros()),
        requests: counters.requests,
        errors: counters.errors,
        bytes: counters.bytes,
        mean_duration_micros: counters.micros / counters.requests.max(1),
        window_micros: open_for.as_micros() as u64,
        dropped_file: drops.file,
        dropped_siem: drops.siem,
    };
    counters.requests = 0;
    counters.errors = 0;
    counters.bytes = 0;
    counters.micros = 0;
    counters.opened = Instant::now();
    summary
}

fn emit(summary: Summary, sinks: &Sinks) {
    tracing::info!(
        requests = summary.requests,
        errors = summary.errors,
        bytes = summary.bytes,
        mean_duration_micros = summary.mean_duration_micros,
        window_micros = summary.window_micros,
        dropped_file = summary.dropped_file,
        dropped_siem = summary.dropped_siem,
        "request summary"
    );
    sinks.offer(Record::RequestSummary(summary));
}

/// Shortens the summary window. **Compiled only under `test-support`.**
///
/// Same reasoning as `Limits::set_response_timeouts`: a witness for a periodic
/// emission cannot wait a minute for it, and a setter that lets a consumer of this
/// crate turn the summary off — or make it write a line per request forever — is a
/// control surface a default build has no reason to carry.
#[cfg(feature = "test-support")]
pub fn set_summary_window(window: Duration) {
    SUMMARY_MILLIS.store(window.as_millis() as u64, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hostile_route_component_cannot_forge_a_log_line() {
        let target = Target::of("/npm/left-pad\r\nINFO forged/1.0.0");
        assert_eq!(target.ecosystem, "npm");
        let package = target.package.expect("a package component");
        assert!(
            !package.contains('\n') && !package.contains('\r'),
            "a newline in a route component must be escaped, not written: {package}"
        );
    }

    #[test]
    fn an_unrecognised_target_contributes_nothing() {
        let target = Target::of("/../../etc/shadow");
        assert_eq!(target.ecosystem, "");
        assert!(target.package.is_none());
        assert!(target.version.is_none());
    }

    #[test]
    fn a_long_component_is_bounded() {
        let long = "a".repeat(4096);
        let target = Target::of(&format!("/npm/{long}"));
        let package = target.package.expect("a package component");
        assert!(
            package.len() <= MAX_LOGGED_TARGET + 2,
            "a route component is bounded before it reaches a log line, got {} characters",
            package.len()
        );
    }
}
