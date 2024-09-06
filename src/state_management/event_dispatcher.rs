use tokio::sync::mpsc;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::models::DockDoorEvent;
use tracing::{info, error};

/// Dispatches events to the appropriate handlers in the dock monitoring system.
pub struct EventDispatcher {
    /// The sender end of a channel for dispatching events.
    event_sender: mpsc::Sender<DockDoorEvent>,
}

impl EventDispatcher {
    /// Creates a new `EventDispatcher`.
    ///
    /// # Arguments
    ///
    /// * `event_sender` - The sender end of a channel for dispatching events.
    ///
    /// # Returns
    ///
    /// A new instance of `EventDispatcher`.
    pub fn new(event_sender: mpsc::Sender<DockDoorEvent>) -> Self {
        Self { event_sender }
    }

    /// Dispatches an event to the appropriate handler.
    ///
    /// This method sends the event through the channel to be processed by the event handler.
    ///
    /// # Arguments
    ///
    /// * `event` - The `DockDoorEvent` to be dispatched.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure of the dispatch operation.
    pub async fn dispatch_event(&self, event: DockDoorEvent) -> DockManagerResult<()> {
        info!("Dispatching event: {:?}", event);
        self.event_sender.send(event).await
            .map_err(|e| {
                error!("Failed to dispatch event: {:?}", e);
                DockManagerError::EventProcessingError(format!("Failed to dispatch event: {}", e))
            })
    }

    /// Dispatches multiple events to the appropriate handlers.
    ///
    /// This method sends multiple events through the channel to be processed by the event handler.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of `DockDoorEvent`s to be dispatched.
    ///
    /// # Returns
    ///
    /// A `DockManagerResult` indicating success or failure of the dispatch operation.
    pub async fn dispatch_events(&self, events: Vec<DockDoorEvent>) -> DockManagerResult<()> {
        info!("Dispatching {} events", events.len());
        for event in events {
            if let Err(e) = self.dispatch_event(event).await {
                error!("Failed to dispatch event: {:?}", e);
                return Err(e);
            }
        }
        Ok(())
    }

    /// Checks if the event channel is still open and able to send events.
    ///
    /// # Returns
    ///
    /// `true` if the channel is open, `false` otherwise.
    pub fn is_channel_open(&self) -> bool {
        !self.event_sender.is_closed()
    }
}