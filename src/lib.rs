//! The injectable application.
//!
//! `App::start` is the single constructor seam: tests build an `AppDeps` with their
//! own implementations and never spawn the binary, and the binary is the only place
//! the production implementations are named.

pub mod artifacts;
pub mod clock;
pub mod concurrency;
pub mod config;
pub mod http;
pub mod npm;
pub mod policy;
pub mod pypi;
pub mod store;
pub mod tasks;
pub mod upstream;

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::artifacts::content::ContentStore;
use crate::artifacts::download::DownloadCoordinator;
use crate::clock::Clock;
use crate::config::Config;
use crate::http::limits::Limits;
use crate::policy::BlocklistSnapshot;
use crate::store::StoreHandle;
use crate::store::cache::MemoryCaches;
use crate::tasks::Tasks;
use crate::tasks::blocklist_poller::{self, Watcher};
use crate::upstream::{OriginSet, Transport};

/// Everything the application is handed from outside. The transport and the origin
/// set are constructor-injected for the same reason the clock is (SPEC §13, finding
/// TEST-01): a test supplies its own, and there is no other way in — no
/// configuration key, flag or environment variable reaches either one.
pub struct AppDeps {
    pub config: Config,
    pub clock: Arc<dyn Clock>,
    pub transport: Arc<dyn Transport>,
    pub origins: OriginSet,
}

/// The shared state every handler sees.
pub struct App {
    pub config: Config,
    pub clock: Arc<dyn Clock>,
    /// The only way this process reaches a registry.
    pub transport: Arc<dyn Transport>,
    /// The origins that transport is allowed to reach, and the URL builder for them.
    pub origins: OriginSet,
    /// The publication point (SPEC §10: "Immutable policy snapshots can use
    /// `ArcSwap`"). SPEC §9 names `blocklist()` as the ordering point of the
    /// revocation boundary: a request that loads after a publish must see it.
    ///
    /// Gate 3 gives these three operations their own `PolicyHandle` type in
    /// `src/policy/blocklist.rs`. They live here until the slice that owns that file
    /// again can lift them out; the seam is the same either way, because every
    /// reader and the one writer already go through these methods.
    policy: ArcSwapOption<BlocklistSnapshot>,
    store: StoreHandle,
    /// Verified artifact bytes on disk (SPEC §9, §10).
    pub content: ContentStore,
    /// One upstream transfer per reference, however many requests want it
    /// (SPEC §9, FLOW-01).
    pub downloads: DownloadCoordinator,
    /// The bounded work SPEC §10 requires: active requests, artifact downloads, and
    /// the two response deadlines.
    pub limits: Limits,
}

impl App {
    /// The blocklist currently in force, whether or not it is still valid. Callers
    /// that judge a package go through `policy::evaluate`, which checks the window
    /// against the same `now` it decides with.
    pub fn blocklist(&self) -> Option<Arc<BlocklistSnapshot>> {
        self.policy.load_full()
    }

    /// Makes `snapshot` the one in force for every request that loads it from here
    /// on. Called only after the snapshot has been committed (SPEC §10).
    pub fn publish_blocklist(&self, snapshot: Arc<BlocklistSnapshot>) {
        self.policy.store(Some(snapshot));
    }

    pub fn blocklist_revision(&self) -> Option<u64> {
        self.policy.load().as_ref().map(|snapshot| snapshot.revision)
    }

    pub fn store(&self) -> &StoreHandle {
        &self.store
    }

    /// Binds the configured address and starts serving. Returns once the listener
    /// is bound, so a caller can read the bound port before the first request.
    ///
    /// The order is SPEC §10's: take the exclusive data-directory lock and recover
    /// the database, publish a persisted blocklist that is still valid, read the
    /// blocklist file once, and only then accept a request. A database that cannot
    /// be recovered is not a reason to refuse to start — it is a reason to start with
    /// readiness false, which is the only way to say so.
    pub async fn start(deps: AppDeps) -> Result<Running, StartupError> {
        let opened = store::startup::open_and_recover(&deps.config.data_dir)
            .await
            .map_err(StartupError::DataDir)?;

        // SPEC §4's `memory_cache_max_bytes`. The caches travel with the store
        // handle, because they exist to keep callers from reaching its queues.
        let caches = Arc::new(MemoryCaches::new(
            deps.config.memory_cache_max_bytes.get(),
        ));

        let (store, store_task) = match opened.connection {
            Ok(connection) => {
                let (store, task) = store::spawn(connection, opened.lock, Arc::clone(&caches));
                (store, Some(task))
            }
            Err(err) => (
                StoreHandle::unusable(err.to_string(), opened.lock, caches),
                None,
            ),
        };

        // SPEC §10: "On startup […] remove incomplete artifact temporary files." A
        // crash must never leave one where a later request could find it.
        let content = ContentStore::new(
            &deps.config.data_dir,
            deps.config.cache_max_bytes.get(),
        );
        content.remove_temp_files();
        let limits = Limits::new(&deps.config);

        let app = Arc::new(App {
            config: deps.config,
            clock: deps.clock,
            transport: deps.transport,
            origins: deps.origins,
            policy: ArcSwapOption::empty(),
            store,
            content,
            downloads: DownloadCoordinator::new(),
            limits,
        });

        restore_blocklist(&app).await;

        // One pass before the listener accepts anything, so an instance with a valid
        // blocklist file is ready on its first request rather than one poll interval
        // later. The poll loop continues from the state this pass leaves behind.
        let mut watcher = Watcher::new();
        blocklist_poller::poll_once(&app, &mut watcher).await;

        let addr = app.config.listen;
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|source| StartupError::Bind { addr, source })?;
        let local_addr = listener.local_addr().map_err(StartupError::Serve)?;

        let shutdown = CancellationToken::new();
        let tasks = tasks::spawn(Arc::clone(&app), shutdown.clone(), watcher);

        let signal = shutdown.clone();
        let server = tokio::spawn({
            let app = Arc::clone(&app);
            async move {
                axum::serve(listener, http::router(app))
                    .with_graceful_shutdown(async move { signal.cancelled().await })
                    .await
            }
        });

        Ok(Running {
            local_addr,
            app,
            shutdown,
            server,
            tasks,
            store_task,
        })
    }
}

/// Publishes the last accepted snapshot when it is still valid (SPEC §8: "Persist
/// the last accepted snapshot so restart can use it while still valid").
///
/// The stored bytes are re-validated rather than trusted: they are validated against
/// *this* `now`, which is what decides that a snapshot persisted yesterday has since
/// expired, and against this build's rules, which is what a schema or rule change
/// between releases would otherwise walk straight past.
async fn restore_blocklist(app: &App) {
    let row = match app.store().load_blocklist().await {
        Ok(Some(row)) => row,
        Ok(None) => return,
        Err(err) => {
            tracing::error!(
                error = %err,
                "the persisted blocklist could not be read; readiness stays false until a \
                 current blocklist is loaded"
            );
            return;
        }
    };

    let now = app.clock.now_utc_micros();
    match BlocklistSnapshot::parse_and_validate(&row.snapshot, now) {
        Ok(snapshot) => {
            tracing::info!(
                revision = snapshot.revision,
                entries = snapshot.entry_count(),
                "the persisted blocklist is still valid and is back in force"
            );
            app.publish_blocklist(Arc::new(snapshot));
        }
        Err(err) => tracing::warn!(
            revision = row.revision,
            error = %err,
            "the persisted blocklist is no longer usable; readiness stays false until a \
             current blocklist is loaded"
        ),
    }
}

/// A started server.
pub struct Running {
    pub local_addr: SocketAddr,
    app: Arc<App>,
    shutdown: CancellationToken,
    server: JoinHandle<std::io::Result<()>>,
    tasks: Tasks,
    /// `None` when the database could not be recovered, so there is no task to wait
    /// for and every store command answers with the reason instead.
    store_task: Option<JoinHandle<()>>,
}

impl Running {
    /// The application this server is running. Tests that assert on state the HTTP
    /// surface does not expose — how many storage commands a request issued, say —
    /// read it from here rather than from a back door in the application itself.
    pub fn app(&self) -> &Arc<App> {
        &self.app
    }

    /// Asks the server to stop accepting and waits for in-flight requests to finish.
    ///
    /// Then, in order: the background loops leave the pass they are in, the last
    /// `App` is dropped so the storage queues close, and the storage task finishes
    /// its current command before it closes the connection and releases the lock.
    pub async fn shutdown(self) -> Result<(), StartupError> {
        self.shutdown.cancel();
        let served = match self.server.await {
            Ok(result) => result.map_err(StartupError::Serve),
            // The task is only ever cancelled by the runtime shutting down, which
            // means the process is going away anyway.
            Err(_) => Ok(()),
        };

        self.tasks.join().await;
        drop(self.app);
        if let Some(task) = self.store_task {
            let _ = task.await;
        }

        served
    }
}

#[derive(Debug)]
pub enum StartupError {
    /// The data directory cannot be created, or another instance holds its lock.
    /// Unlike a database that will not recover, there is nothing left to run.
    DataDir(store::startup::StartupError),
    Bind {
        addr: SocketAddr,
        source: std::io::Error,
    },
    Serve(std::io::Error),
}

impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartupError::DataDir(err) => write!(f, "{err}"),
            StartupError::Bind { addr, source } => write!(f, "cannot bind {addr}: {source}"),
            StartupError::Serve(source) => write!(f, "server stopped: {source}"),
        }
    }
}

impl std::error::Error for StartupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StartupError::DataDir(err) => Some(err),
            StartupError::Bind { source, .. } | StartupError::Serve(source) => Some(source),
        }
    }
}
