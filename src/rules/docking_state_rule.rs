use crate::models::{DockDoor, DockDoorEvent, TrailerState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry};

/// An analysis rule that logs trailer state changes (docked/undocked).
pub struct TrailerStateChangeRule;

impl AnalysisRule for TrailerStateChangeRule {
    /// Applies the rule to a dock door event, generating a log entry if the event is a trailer state change
    ///
    /// The method checks if the provided event is a `TrailerStateChangedEvent`. If so, it creates a `LogEntry::TrailerStateChange`
    /// with relevant details like the timestamp, plant ID, door name, shipment ID, event type (TRAILER_DOCKED or TRAILER_UNDOCKED),
    /// success status, notes about the state change, severity, previous state, and previous state timestamp
    /// The log entry is wrapped in an `AnalysisResult::Log` and returned in a vector
    /// If the event is not a trailer state change, an empty vector is returned
    ///
    /// # Arguments
    ///
    /// * `door`: A reference to the `DockDoor` object the event is associated with
    /// * `event`: A reference to the `DockDoorEvent` to be analyzed
    ///
    /// # Returns
    ///
    /// A vector containing an `AnalysisResult::Log` if the event is a trailer state change, otherwise an empty vector
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        if let DockDoorEvent::TrailerStateChanged(e) = event {
            let event_type = match e.new_state {
                TrailerState::Docked => "TRAILER_DOCKED",
                TrailerState::Undocked => "TRAILER_UNDOCKED",
            }.to_string();

            let notes = format!("Trailer state changed from {:?} to {:?}", e.old_state, e.new_state);

            let log_entry = LogEntry::TrailerStateChange {
                log_dttm: e.timestamp,
                plant: door.plant_id.clone(),
                door_name: e.dock_name.clone(),
                shipment_id: door.assigned_shipment.current_shipment.clone(),
                event_type: event_type.clone(),
                success: true,
                notes: notes.clone(),
                severity: 0,
                previous_state: Some(format!("{:?}", e.old_state)),
                previous_state_dttm: Some(e.timestamp),
            };


            vec![AnalysisResult::Log(log_entry)]
        } else {
            vec![]
        }
    }
}