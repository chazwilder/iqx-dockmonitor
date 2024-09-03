use crate::models::{DockDoor, DockDoorEvent, TrailerState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry};
use chrono::Local;

pub struct TrailerUndockingRule;

impl AnalysisRule for TrailerUndockingRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let mut results = Vec::new();

        if let DockDoorEvent::TrailerStateChanged(e) = event {
            if e.new_state == TrailerState::Undocked && e.old_state == TrailerState::Docked {
                let log_entry = LogEntry::DockingTime {
                    log_dttm: Local::now().naive_local(),
                    plant: door.plant_id.clone(),
                    door_name: door.dock_name.clone(),
                    shipment_id: door.assigned_shipment.current_shipment.clone(),
                    event_type: "TRAILER_UNDOCKING".to_string(),
                    success: true,
                    notes: "Trailer undocked successfully".to_string(),
                    severity: 0,
                    previous_state: Some(format!("{:?}", e.old_state)),
                    previous_state_dttm: Some(e.timestamp),
                };

                results.push(AnalysisResult::Log(log_entry));
            }
        }

        results
    }
}