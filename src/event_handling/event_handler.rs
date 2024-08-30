//! # Event Handling

//! This module handles the processing of events in the dock door management system. It receives events from a queue, analyzes them using a `ContextAnalyzer`, 
//! updates the state of dock doors through a `DockDoorStateManager`, and persists relevant events to the database.

use crate::models::{DockDoorEvent, DbInsert};
use crate::state_management::DockDoorStateManager;
use crate::analysis::ContextAnalyzer;
use crate::errors::{DockManagerResult, DockManagerError};
use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use tracing::{info, error, debug};

/// The `EventHandler` is responsible for processing events in the dock door management system.
pub struct EventHandler {
    /// A queue for receiving `DockDoorEvent`s.
    event_queue: Mutex<mpsc::Receiver<DockDoorEvent>>,
    /// The state manager responsible for maintaining the state of dock doors.
    state_manager: Arc<DockDoorStateManager>,
    /// The context analyzer used to analyze events and generate insights.
    context_analyzer: Arc<ContextAnalyzer>,
}

impl EventHandler {
    /// Creates a new `EventHandler`.
    /// 
    /// # Arguments
    /// * `event_queue`: The receiver end of a channel to receive `DockDoorEvent`s.
    /// * `state_manager`: The `DockDoorStateManager` to interact with for state updates.
    /// * `context_analyzer`: The `ContextAnalyzer` to use for event analysis.
    pub fn new(
        event_queue: mpsc::Receiver<DockDoorEvent>,
        state_manager: Arc<DockDoorStateManager>,
        context_analyzer: Arc<ContextAnalyzer>,
    ) -> Self {
        Self {
            event_queue: Mutex::new(event_queue),
            state_manager,
            context_analyzer,
        }
    }

    /// Runs the event handler, continuously processing events from the queue
    /// until the queue is closed or an error occurs
    pub async fn run(&self) -> DockManagerResult<()> {
        info!("EventHandler started");
        while let Some(event) = self.event_queue.lock().await.recv().await {
            if let Err(e) = self.process_event(event).await {
                error!("Error processing event: {:?}", e);
            }
        }
        info!("EventHandler stopped");
        Ok(())
    }

    /// Processes an event and potentially generates database insert events
    /// 
    /// This method is primarily used for testing purposes
    ///
    /// # Arguments
    /// 
    /// * `event`: The `DockDoorEvent` to process.
    /// 
    /// # Returns
    /// 
    /// A `DockManagerResult` containing a vector of `DbInsert` events if successful, 
    /// or an error if the event processing fails.
    pub async fn send_event(&self, event: DockDoorEvent) -> DockManagerResult<Vec<DbInsert>> {
        self.process_event(event).await
    }

    /// Processes a single dock door event
    /// 
    /// This method retrieves the associated dock door, analyzes the event using the `ContextAnalyzer`,
    /// handles any resulting state transitions or alerts, updates the door's state, 
    /// and inserts any generated `DbInsert` events into the database
    ///
    /// # Arguments
    ///
    /// * `event`: The `DockDoorEvent` to be processed
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DbInsert>)`: A vector of `DbInsert` events generated during processing
    /// * `Err(DockManagerError)`: If there's an error processing the event or interacting with the database or state manager
    async fn process_event(&self, event: DockDoorEvent) -> DockManagerResult<Vec<DbInsert>> {
        debug!("Processing event: {:?}", event);

        let door_name = event.get_dock_name();
        let mut door = self.state_manager.get_door(door_name).await
            .ok_or_else(|| DockManagerError::DoorNotFound(door_name.to_string()))?;

        let analysis_results = self.context_analyzer.analyze(&door, &event).await;

        let mut db_events = Vec::new();
        for result in analysis_results {
            match result {
                crate::analysis::AnalysisResult::StateTransition(new_state) => {
                    door.door_state = new_state;
                },
                crate::analysis::AnalysisResult::Log(log_entry) => {
                    let db_insert = DbInsert::from_log_entry(&log_entry);
                    db_events.push(db_insert);
                },
                crate::analysis::AnalysisResult::Alert(alert) => {
                    info!("{:?}", alert)
                }
                crate::analysis::AnalysisResult::DbInsert(db_insert) => {
                    db_events.push(db_insert);
                }
            }
        }

        door.handle_event(&event)?;

        self.state_manager.update_door(door).await?;

        if !db_events.is_empty() {
            self.insert_db_events(&db_events).await?;
        }

        Ok(db_events)
    }

    /// Inserts a batch of database events
    ///
    /// This method iterates through the provided `DbInsert` events and attempts to insert each one into the database
    /// using the state manager's `insert_db_event` method
    ///
    /// # Arguments
    ///
    /// * `db_events`: A slice of `DbInsert` events to be inserted
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` if all events were inserted successfully
    /// * The first encountered `DockManagerError` if any insertion fails
    pub async fn insert_db_events(&self, db_events: &[DbInsert]) -> DockManagerResult<()> {
        for event in db_events {
            self.state_manager.insert_db_event(event.clone()).await?;
        }
        Ok(())
    }
}