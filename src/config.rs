//! The configuration file (SPEC §4).
//!
//! Every rejection rule lives here, so `serve` and the validation commands cannot
//! diverge: both build the same `Config` through the same conversion.

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::num::{NonZeroU32, NonZeroU64};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use url::Url;

/// The validated configuration. The `NonZero*` types make "zero polling intervals
/// and zero capacity limits are invalid" a type error rather than a check someone
/// can forget; `cooldown_seconds`, `metadata_ttl_seconds` and
/// `metadata_max_age_seconds` stay plain `u64` because zero is meaningful for all
/// three.
#[derive(Clone, Debug)]
pub struct Config {
    pub listen: SocketAddr,
    pub public_url: Url,
    pub data_dir: PathBuf,
    pub blocklist_file: PathBuf,
    pub cooldown_seconds: u64,
    pub metadata_ttl_seconds: u64,
    /// SPEC rev 3 §10: how old a stored project snapshot may become since its last
    /// FULL fetch, whatever upstream keeps answering `304` to. Zero disables the
    /// ceiling and restores unbounded revalidation.
    ///
    /// Not a `NonZeroU64` even though zero is the "off" value: the rule that makes
    /// it safe is a relation to `metadata_ttl_seconds`, and a type cannot express a
    /// cross-field relation.
    pub metadata_max_age_seconds: u64,
    pub blocklist_poll_seconds: NonZeroU64,
    pub cache_max_bytes: NonZeroU64,
    pub memory_cache_max_bytes: NonZeroU64,
    pub max_artifact_bytes: NonZeroU64,
    pub max_metadata_bytes: NonZeroU64,
    pub max_blocklist_bytes: NonZeroU64,
    pub max_upstream_requests: NonZeroU32,
    pub max_artifact_downloads: NonZeroU32,
    pub max_active_requests: NonZeroU32,
    pub max_references_per_project: NonZeroU32,
}

/// The default for the one key SPEC §4 does not list. It is a Gate 3 addition
/// adopted from threat TM-1: a reference count cap bounds the duration of a single
/// project-refresh transaction independently of the byte cap.
const DEFAULT_MAX_REFERENCES_PER_PROJECT: u32 = 20_000;

/// The ceiling a file that predates SPEC revision 3 gets: one day, which is what
/// `config.sample.toml` ships. A file with no opinion inherits a bound rather than
/// the unbounded staleness the ceiling exists to close.
const DEFAULT_METADATA_MAX_AGE_SECONDS: u64 = 86_400;

/// The file as written, before validation. Unknown keys are rejected so a typo in
/// an operator's configuration is an error rather than a silently ignored line.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    listen: String,
    public_url: String,
    data_dir: PathBuf,
    blocklist_file: PathBuf,
    cooldown_seconds: u64,
    metadata_ttl_seconds: u64,
    #[serde(default = "default_metadata_max_age_seconds")]
    metadata_max_age_seconds: u64,
    blocklist_poll_seconds: u64,
    cache_max_bytes: u64,
    memory_cache_max_bytes: u64,
    max_artifact_bytes: u64,
    max_metadata_bytes: u64,
    max_blocklist_bytes: u64,
    max_upstream_requests: u32,
    max_artifact_downloads: u32,
    max_active_requests: u32,
    #[serde(default = "default_max_references_per_project")]
    max_references_per_project: u32,
}

fn default_max_references_per_project() -> u32 {
    DEFAULT_MAX_REFERENCES_PER_PROJECT
}

fn default_metadata_max_age_seconds() -> u64 {
    DEFAULT_METADATA_MAX_AGE_SECONDS
}

/// The keys every configuration file must carry, in the order SPEC §4 lists them.
/// They are named here as well as in `RawConfig` so that a missing key and an
/// unknown one can be reported as themselves: serde reports both as a
/// deserialisation failure, and "invalid TOML" is not a reason an operator can act
/// on. `tests/config_validation.rs` deletes each key of the shipped sample in turn,
/// so this list cannot silently drift from `RawConfig`.
const REQUIRED_KEYS: &[&str] = &[
    "listen",
    "public_url",
    "data_dir",
    "blocklist_file",
    "cooldown_seconds",
    "metadata_ttl_seconds",
    "blocklist_poll_seconds",
    "cache_max_bytes",
    "memory_cache_max_bytes",
    "max_artifact_bytes",
    "max_metadata_bytes",
    "max_blocklist_bytes",
    "max_upstream_requests",
    "max_artifact_downloads",
    "max_active_requests",
];

/// Keys with a default, which a file may leave out.
const OPTIONAL_KEYS: &[&str] = &["max_references_per_project", "metadata_max_age_seconds"];

impl Config {
    pub fn from_toml_str(text: &str) -> Result<Config, ConfigError> {
        // Three stages, so each failure keeps its own name: the document must be
        // TOML, then its key set must be exactly ours, then its values must convert.
        let table: toml::Table = toml::from_str(text).map_err(ConfigError::Syntax)?;
        check_keys(&table)?;
        let raw: RawConfig = toml::from_str(text).map_err(ConfigError::Syntax)?;
        raw.validate()
    }

    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Config::from_toml_str(&text)
    }
}

impl RawConfig {
    fn validate(self) -> Result<Config, ConfigError> {
        let listen = self
            .listen
            .parse::<SocketAddr>()
            .map_err(|err| invalid("listen", err.to_string()))?;

        let public_url =
            Url::parse(&self.public_url).map_err(|err| invalid("public_url", err.to_string()))?;
        if public_url.scheme() != "https" {
            return Err(invalid(
                "public_url",
                "must use the https scheme".to_owned(),
            ));
        }
        if public_url.query().is_some() || public_url.fragment().is_some() {
            return Err(invalid(
                "public_url",
                "must not carry a query or fragment".to_owned(),
            ));
        }
        if public_url.path() != "/" && !public_url.path().is_empty() {
            return Err(invalid(
                "public_url",
                "must not carry a path prefix".to_owned(),
            ));
        }

        if !self.data_dir.is_absolute() {
            return Err(invalid("data_dir", "must be an absolute path".to_owned()));
        }
        if !self.blocklist_file.is_absolute() {
            return Err(invalid(
                "blocklist_file",
                "must be an absolute path".to_owned(),
            ));
        }

        // SPEC rev 3 §10. A ceiling beneath the revalidation interval would expire
        // every copy before it could ever be revalidated: a configuration that looks
        // stricter and is in fact a self-inflicted outage. Zero is the "off" value
        // and is exempt.
        if self.metadata_max_age_seconds != 0
            && self.metadata_max_age_seconds < self.metadata_ttl_seconds
        {
            return Err(invalid(
                "metadata_max_age_seconds",
                format!(
                    "must not be below metadata_ttl_seconds ({}); use 0 to disable the ceiling",
                    self.metadata_ttl_seconds
                ),
            ));
        }

        let cache_max_bytes = nonzero_u64("cache_max_bytes", self.cache_max_bytes)?;
        let max_artifact_bytes = nonzero_u64("max_artifact_bytes", self.max_artifact_bytes)?;
        if max_artifact_bytes > cache_max_bytes {
            return Err(invalid(
                "max_artifact_bytes",
                "must not exceed cache_max_bytes".to_owned(),
            ));
        }

        Ok(Config {
            listen,
            public_url,
            data_dir: self.data_dir,
            blocklist_file: self.blocklist_file,
            cooldown_seconds: self.cooldown_seconds,
            metadata_ttl_seconds: self.metadata_ttl_seconds,
            metadata_max_age_seconds: self.metadata_max_age_seconds,
            blocklist_poll_seconds: nonzero_u64(
                "blocklist_poll_seconds",
                self.blocklist_poll_seconds,
            )?,
            cache_max_bytes,
            memory_cache_max_bytes: nonzero_u64(
                "memory_cache_max_bytes",
                self.memory_cache_max_bytes,
            )?,
            max_artifact_bytes,
            max_metadata_bytes: nonzero_u64("max_metadata_bytes", self.max_metadata_bytes)?,
            max_blocklist_bytes: nonzero_u64("max_blocklist_bytes", self.max_blocklist_bytes)?,
            max_upstream_requests: nonzero_u32(
                "max_upstream_requests",
                self.max_upstream_requests,
            )?,
            max_artifact_downloads: nonzero_u32(
                "max_artifact_downloads",
                self.max_artifact_downloads,
            )?,
            max_active_requests: nonzero_u32("max_active_requests", self.max_active_requests)?,
            max_references_per_project: nonzero_u32(
                "max_references_per_project",
                self.max_references_per_project,
            )?,
        })
    }
}

/// An unknown key is reported before a missing one: a typo produces both, and the
/// misspelling is the half an operator can fix.
fn check_keys(table: &toml::Table) -> Result<(), ConfigError> {
    for key in table.keys() {
        let known = REQUIRED_KEYS.contains(&key.as_str()) || OPTIONAL_KEYS.contains(&key.as_str());
        if !known {
            return Err(ConfigError::UnknownKey(key.clone()));
        }
    }
    for key in REQUIRED_KEYS {
        if !table.contains_key(*key) {
            return Err(ConfigError::MissingKey(key));
        }
    }
    Ok(())
}

fn invalid(key: &'static str, reason: String) -> ConfigError {
    ConfigError::Invalid { key, reason }
}

fn nonzero_u64(key: &'static str, value: u64) -> Result<NonZeroU64, ConfigError> {
    NonZeroU64::new(value).ok_or_else(|| invalid(key, "must not be zero".to_owned()))
}

fn nonzero_u32(key: &'static str, value: u32) -> Result<NonZeroU32, ConfigError> {
    NonZeroU32::new(value).ok_or_else(|| invalid(key, "must not be zero".to_owned()))
}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Syntax(toml::de::Error),
    /// A key this release does not know. SPEC §11's TEST-01 rule depends on this
    /// refusal: no configuration key can relax a decision, so a key that looks like
    /// one must not be quietly ignored.
    UnknownKey(String),
    MissingKey(&'static str),
    Invalid {
        key: &'static str,
        reason: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            ConfigError::Syntax(err) => write!(f, "invalid TOML: {err}"),
            ConfigError::UnknownKey(key) => write!(f, "unknown key `{key}`"),
            ConfigError::MissingKey(key) => write!(f, "missing key `{key}`"),
            ConfigError::Invalid { key, reason } => write!(f, "invalid `{key}`: {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Syntax(err) => Some(err),
            ConfigError::UnknownKey(_)
            | ConfigError::MissingKey(_)
            | ConfigError::Invalid { .. } => None,
        }
    }
}
