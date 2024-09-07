use tokio::sync::Notify;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, error};
use crate::errors::DockManagerResult;
use crate::state_management::database_event_manager::DatabaseEventManager;

/// Manages the lifecycle of the state manager in the dock monitoring system.
pub struct StateManagerLifecycle {
    /// Signal for triggering and waiting for shutdown.
    shutdown_signal: Arc<Notify>,
    /// Boolean flag to indicate if shutdown has been triggered.
    shutdown_triggered: Arc<AtomicBool>,
    /// Reference to the database event manager for flushing events during shutdown.
    db_event_manager: Arc<DatabaseEventManager>,
}

impl StateManagerLifecycle {
    /// Creates a new `StateManagerLifecycle`.
    ///
    /// # Arguments
    ///
    /// * `db_event_manager` - A reference to the `DatabaseEventManager` for handling shutdown operations.
    ///
    /// # Returns
    ///
    /// A new instance of `StateManagerLifecycle`.
    pub fn new(db_event_manager: Arc<DatabaseEventManager>) -> Self {
        Self {
            shutdown_signal: Arc::new(Notify::new()),
            shutdown_triggered: Arc::new(AtomicBool::new(false)),
            db_event_manager,
        }
    }

    /// Waits for the shutdown signal.
    ///
    /// This method blocks until the shutdown signal is triggered.
    pub async fn wait_for_shutdown(&self) {
        info!("Waiting for shutdown signal");
        self.shutdown_signal.notified().await;
        info!("Shutdown signal received");
    }

    /// Triggers the shutdown signal.
    ///
    /// This method notifies all tasks waiting on the shutdown signal to begin their shutdown process.
    pub fn trigger_shutdown(&self) {
        info!("Triggering shutdown");
        self.shutdown_triggered.store(true, Ordering::SeqCst);
        self.shutdown_signal.notify_waiters();
    }

    /// Performs cleanup operations during shutdown.
    ///
    /// This method is responsible for any necessary cleanup tasks, such as flushing remaining database events.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure of the cleanup process.
    pub async fn cleanup(&self) -> DockManagerResult<()> {
        info!("Starting cleanup process");

        // Flush any remaining database events
        if let Err(e) = self.db_event_manager.flush_events().await {
            error!("Error flushing database events during cleanup: {:?}", e);
            // Decide whether to return the error or continue with other cleanup tasks
        }

        info!("Cleanup process completed");
        Ok(())
    }

    /// Checks if the shutdown signal has been triggered.
    ///
    /// # Returns
    ///
    /// `true` if the shutdown signal has been triggered, `false` otherwise.
    pub fn is_shutdown_triggered(&self) -> bool {
        self.shutdown_triggered.load(Ordering::SeqCst)
    }

    /// Resets the shutdown signal.
    ///
    /// This method can be used to reset the shutdown state, allowing the system to be restarted.
    /// It should be used with caution and only in specific scenarios where a reset is appropriate.
    pub fn reset_shutdown_signal(&self) {
        info!("Resetting shutdown signal");
        self.shutdown_triggered.store(false, Ordering::SeqCst);
        // Note: We don't need to reset the Notify here as it automatically resets after notifying
    }
}