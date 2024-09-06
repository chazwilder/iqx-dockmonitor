use tokio::sync::RwLock;
use crate::models::DbInsert;
use crate::errors::DockManagerError;
use crate::services::db::DatabaseService;
use std::sync::Arc;
use tracing::{info, error};

/// Manages the collection and processing of database events for the dock monitoring system.
pub struct DatabaseEventManager {
    /// The queue of database events waiting to be processed.
    db_events: RwLock<Vec<DbInsert>>,
    /// The maximum number of events to accumulate before automatically flushing to the database.
    batch_size: usize,
    /// The database service used for inserting events.
    db_service: Arc<DatabaseService>,
}

impl DatabaseEventManager {
    /// Creates a new `DatabaseEventManager`.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - The maximum number of events to accumulate before automatically flushing.
    /// * `db_service` - A reference to the `DatabaseService` for database operations.
    ///
    /// # Returns
    ///
    /// A new instance of `DatabaseEventManager`.
    pub fn new(batch_size: usize, db_service: Arc<DatabaseService>) -> Self {
        Self {
            db_events: RwLock::new(Vec::new()),
            batch_size,
            db_service,
        }
    }

    /// Adds a new database event to the queue.
    ///
    /// If the number of queued events reaches the batch size, it automatically
    /// triggers a flush operation.
    ///
    /// # Arguments
    ///
    /// * `event` - The `DbInsert` event to be added to the queue.
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the operation.
    pub async fn add_event(&self, event: DbInsert) -> Result<(), DockManagerError> {
        let mut events = self.db_events.write().await;
        events.push(event);

        if events.len() >= self.batch_size {
            drop(events); // Release the write lock before flushing
            self.flush_events().await?;
        }

        Ok(())
    }

    /// Flushes all queued events to the database.
    ///
    /// This method is called automatically when the batch size is reached,
    /// or it can be called manually to force a flush operation.
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the flush operation.
    pub async fn flush_events(&self) -> Result<(), DockManagerError> {
        let mut events = self.db_events.write().await;
        if events.is_empty() {
            return Ok(());
        }

        let events_to_flush = std::mem::take(&mut *events);
        drop(events); // Release the write lock before database operation

        info!("Flushing {} database events", events_to_flush.len());
        match self.db_service.insert_dock_door_events(events_to_flush).await {
            Ok(_) => {
                info!("Successfully flushed database events");
                Ok(())
            },
            Err(e) => {
                error!("Failed to flush database events: {:?}", e);
                // In case of failure, we might want to re-queue the events or implement a retry mechanism
                Err(e)
            }
        }
    }

    /// Retrieves and clears all currently queued database events.
    ///
    /// This method is useful for getting a snapshot of current events,
    /// for example during shutdown or for manual processing.
    ///
    /// # Returns
    ///
    /// A vector of all queued `DbInsert` events.
    pub async fn get_and_clear_events(&self) -> Vec<DbInsert> {
        let mut events = self.db_events.write().await;
        std::mem::take(&mut *events)
    }

    /// Returns the current number of queued database events.
    ///
    /// # Returns
    ///
    /// The number of events currently in the queue.
    pub async fn queue_size(&self) -> usize {
        let events = self.db_events.read().await;
        events.len()
    }
}