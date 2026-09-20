//! The CLI. Builds the production `AppDeps`, installs the log subscriber, and maps
//! failures to exit codes. No policy or protocol logic lives here.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use package_firewall::clock::{Clock, SystemClock};
use package_firewall::config::Config;
use package_firewall::policy::blocklist;
use package_firewall::upstream::{OriginSet, ReqwestTransport};
use package_firewall::{App, AppDeps};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "package-firewall",
    version,
    about = "A filtering proxy for npm and PyPI that withholds packages until they are eligible."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve filtered npm and PyPI metadata and verified artifacts.
    Serve {
        /// Path to the TOML configuration file.
        #[arg(long, value_name = "PATH")]
        config: PathBuf,
    },
    /// Validate a configuration file. Exits non-zero on the first problem, naming
    /// it, and changes nothing.
    CheckConfig {
        /// Path to the TOML configuration file.
        #[arg(value_name = "PATH", required_unless_present = "config")]
        path: Option<PathBuf>,
        /// The same path in the `--config` form SPEC §4 spells the command with.
        #[arg(long, value_name = "PATH", conflicts_with = "path")]
        config: Option<PathBuf>,
    },
    /// Validate a blocklist snapshot. Exits non-zero on the first problem, naming
    /// it, and changes nothing.
    CheckBlocklist {
        /// Path to the JSON blocklist snapshot.
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    match cli.command {
        Command::Serve { config } => serve(&config).await,
        Command::CheckConfig { path, config } => {
            // clap guarantees exactly one of the two forms is present.
            match path.or(config) {
                Some(path) => check_config(&path),
                None => ExitCode::FAILURE,
            }
        }
        Command::CheckBlocklist { path } => check_blocklist(&path),
    }
}

/// SPEC §4: the validation commands exit non-zero on failure and do not modify
/// state. They read one file and print one line; nothing here writes anywhere.
fn check_config(path: &Path) -> ExitCode {
    match Config::load(path) {
        Ok(config) => {
            println!(
                "{}: valid configuration (listen {}, cooldown {}s)",
                path.display(),
                config.listen,
                config.cooldown_seconds
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{}: {err}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn check_blocklist(path: &Path) -> ExitCode {
    // The command is handed a path and no configuration, so it validates against the
    // documented size limit rather than an operator's own `max_blocklist_bytes`.
    let now = SystemClock.now_utc_micros();
    match blocklist::load_file(path, blocklist::DEFAULT_MAX_BLOCKLIST_BYTES, now) {
        Ok(snapshot) => {
            println!(
                "{}: valid blocklist (revision {}, {} entries, expires {})",
                path.display(),
                snapshot.revision,
                snapshot.entry_count(),
                jiff::Timestamp::from_microsecond(snapshot.expires_at_micros)
                    .map(|timestamp| timestamp.to_string())
                    .unwrap_or_else(|_| snapshot.expires_at_micros.to_string()),
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{}: {err}", path.display());
            ExitCode::FAILURE
        }
    }
}

async fn serve(config_path: &Path) -> ExitCode {
    let config = match Config::load(config_path) {
        Ok(config) => config,
        Err(err) => {
            tracing::error!(path = %config_path.display(), error = %err, "invalid configuration");
            return ExitCode::FAILURE;
        }
    };

    let running = match App::start(AppDeps {
        config,
        clock: Arc::new(SystemClock),
        transport: Arc::new(ReqwestTransport::production()),
        origins: OriginSet::production(),
    })
    .await
    {
        Ok(running) => running,
        Err(err) => {
            tracing::error!(error = %err, "cannot start");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(addr = %running.local_addr, "listening");

    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %err, "cannot listen for the shutdown signal");
        return ExitCode::FAILURE;
    }

    tracing::info!("shutting down");
    match running.shutdown().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "unclean shutdown");
            ExitCode::FAILURE
        }
    }
}
