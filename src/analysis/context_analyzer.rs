use chrono::{Duration, NaiveDateTime};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::models::{DoorState, DockDoorEvent, DockDoor, DbInsert};

/// The result of applying an analysis rule to a dock door event
#[derive(Debug, Clone)]
pub enum AnalysisResult {
    /// An alert to be triggered, with the specific alert type
    Alert(AlertType),
    /// A state transition for the dock door
    StateTransition(DoorState),
    /// A log entry to be recorded
    Log(LogEntry),
    /// A database insert operation to be performed
    DbInsert(DbInsert),
}

/// The different types of alerts that can be generated by analysis rules
#[derive(Debug, Clone)]
pub enum AlertType {
    /// The docking time has exceeded a defined threshold
    LongDockingTime(Duration),
    /// Manual intervention was required
    ManualIntervention,
    /// A trailer is being held hostage at the dock
    TrailerHostage {
        door_name: String,
        shipment_id: Option<String>,
        duration: Duration,
    },
    /// A trailer is departing under unsafe conditions
    UnsafeDeparture,
    /// Manual mode has been activated while a trailer is at the door
    ManualModeAlert {
        door_name: String,
        shipment_id: Option<String>,
    },
    /// A new shipment is assigned while the previous trailer is still present
    NewShipmentPreviousTrailerPresent {
        dock_name: String,
        new_shipment: String,
        previous_shipment: Option<String>,
        timestamp: NaiveDateTime
    },
    /// Manual intervention has timed out without resolving the issue
    ManualInterventionTimeout {
        dock_name: String,
        shipment_id: String,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime
    },
    /// A door has been suspended for an extended period
    SuspendedDoor {
        door_name: String,
        duration: Duration,
        shipment_id: Option<String>,
    },
    /// A loading process has taken too long to start
    LongLoadingStart {
        door_name: String,
        shipment_id: String,
        duration: Duration,
    },
    /// A shipment has started loading but the dock is not ready
    ShipmentStartedLoadNotReady {
        door_name: String,
        shipment_id: String,
        reason: String,
    },
    /// A trailer pattern issue has been detected
    TrailerPatternIssue {
        door_name: String,
        issue: String,
        severity: i32,
        shipment_id: Option<String>,
    },
    /// A trailer has been docked but loading hasn't started
    TrailerDockedNotStarted {
        door_name: String,
        duration: Duration,
    },
    TrailerDocked {
        door_name: String,
        shipment_id: Option<String>,
        timestamp: NaiveDateTime,
    },
    DockReady {
        door_name: String,
        shipment_id: Option<String>,
        timestamp: NaiveDateTime,
    },
    TrailerUndocked {
        door_name: String,
        shipment_id: Option<String>,
        timestamp: NaiveDateTime,
    },
}

/// Represents different types of log entries that can be generated by analysis rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntry {
    /// Logs the docking time for a shipment
    DockingTime {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs the activation of manual mode for a dock door
    ManualModeActivated {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a change in the loading status of a shipment
    LoadingStatusChange {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs when a shipment is unassigned from a door
    ShipmentUnassigned {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs when a shipment is assigned to a door
    ShipmentAssigned {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs when a new shipment is assigned to a door while the previous trailer is still present
    NewShipmentPreviousTrailerPresent {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs the start of a manual intervention on a dock door
    ManualInterventionStarted {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a successful manual intervention on a dock door
    ManualInterventionSuccess {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a failed manual intervention on a dock door
    ManualInterventionFailure {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a change in the trailer's state at a dock door
    TrailerStateChange {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a suspended door event
    SuspendedDoor {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a long loading start event
    LongLoadingStart {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a trailer hostage event
    TrailerHostage {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a shipment started but load not ready event
    ShipmentStartedLoadNotReady {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    /// Logs a trailer pattern issue event
    TrailerPatternIssue {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
    TrailerUndocked {
        log_dttm: NaiveDateTime,
        plant: String,
        door_name: String,
        shipment_id: Option<String>,
        event_type: String,
        success: bool,
        notes: String,
        severity: i32,
        previous_state: Option<String>,
        previous_state_dttm: Option<NaiveDateTime>,
    },
}

/// Defines the interface for analysis rules that can be applied to dock door events
pub trait AnalysisRule: Send + Sync {
    /// Applies the analysis rule to a dock door and an event, potentially generating `AnalysisResult`s
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult>;
}

/// Analyzes dock door events in context using a set of rules
#[derive(Default, Clone)]
pub struct ContextAnalyzer {
    /// The collection of analysis rules to apply
    rules: Vec<Arc<dyn AnalysisRule>>,
}

impl ContextAnalyzer {
    /// Creates a new `ContextAnalyzer` with no rules initially
    pub fn new() -> Self {
        ContextAnalyzer { rules: Vec::new() }
    }

    /// Adds an analysis rule to the analyzer
    pub fn add_rule(&mut self, rule: Arc<dyn AnalysisRule>) {
        self.rules.push(rule);
    }

    /// Analyzes a dock door event using the registered rules
    pub async fn analyze(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let results: Vec<AnalysisResult>  = self.rules
            .iter()
            .flat_map(|rule| {
                let rule_results = rule.apply(dock_door, event);
                rule_results
            })
            .collect();
        results
    }
}

/// Creates a default `ContextAnalyzer` with no rules
pub fn create_default_analyzer() -> ContextAnalyzer {
    ContextAnalyzer::new()
}