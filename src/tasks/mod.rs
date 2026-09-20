//! The background loops. Nothing depends on this module; it depends on the rest.
//!
//! Each loop is handed the same `CancellationToken`, so a graceful shutdown stops
//! all of them at once and `Tasks::shutdown` can wait for each to finish the pass it
//! was in.

pub mod blocklist_poller;
pub mod maintenance;

use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::App;

/// The loops this build runs: the blocklist poller and the maintenance pass (batched
/// access times, eviction and the WAL checkpoint).
pub struct Tasks {
    handles: Vec<JoinHandle<()>>,
}

/// Spawns every background loop. `watcher` carries whatever the startup pass already
/// learned about the blocklist file, so the first scheduled pass does not re-read a
/// file it has just read.
pub fn spawn(
    app: Arc<App>,
    shutdown: CancellationToken,
    watcher: blocklist_poller::Watcher,
) -> Tasks {
    let handles = vec![
        tokio::spawn(blocklist_poller::run(
            Arc::clone(&app),
            shutdown.clone(),
            watcher,
        )),
        tokio::spawn(maintenance::run(app, shutdown)),
    ];

    Tasks { handles }
}

impl Tasks {
    /// Waits for every loop to leave the pass it is in. The caller cancels the token
    /// first; a loop that has already stopped joins immediately.
    pub async fn join(self) {
        for handle in self.handles {
            // A task is only ever cancelled by the runtime going away, which means
            // the process is going away with it.
            let _ = handle.await;
        }
    }
}
