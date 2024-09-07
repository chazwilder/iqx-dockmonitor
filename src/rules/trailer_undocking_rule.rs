use crate::models::{DockDoor, DockDoorEvent, TrailerState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry, AlertType};
use chrono::{Local};
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerUndockingRuleConfig {
    // Add any configuration parameters if needed
}

pub struct TrailerUndockingRule {
    config: TrailerUndockingRuleConfig,
}

impl TrailerUndockingRule {
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: TrailerUndockingRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse TrailerUndockingRule configuration");
        Self { config: parsed_config }
    }

    fn generate_undocking_results(&self, door: &DockDoor, timestamp: chrono::NaiveDateTime, previous_state: &str) -> Vec<AnalysisResult> {
        let mut results = Vec::new();
        let _con = self.config.clone();

        let log_entry = LogEntry::TrailerUndocked {
            log_dttm: Local::now().naive_local(),
            plant: door.plant_id.clone(),
            door_name: door.dock_name.clone(),
            shipment_id: door.assigned_shipment.current_shipment.clone(),
            event_type: "TRAILER_UNDOCKING".to_string(),
            success: true,
            notes: "Trailer undocked successfully".to_string(),
            severity: 0,
            previous_state: Some(previous_state.to_string()),
            previous_state_dttm: Some(timestamp),
        };

        debug!("TrailerUndockingRule: Generated undocking log entry: {:?}", log_entry);
        results.push(AnalysisResult::Log(log_entry));

        // Add an alert for the undocking event
        results.push(AnalysisResult::Alert(AlertType::TrailerUndocked {
            door_name: door.dock_name.clone(),
            shipment_id: door.assigned_shipment.current_shipment.clone(),
            timestamp,
        }));

        results
    }
}

impl AnalysisRule for TrailerUndockingRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::TrailerStateChanged(e) => {
                if e.new_state == TrailerState::Undocked && e.old_state == TrailerState::Docked {
                    self.generate_undocking_results(door, e.timestamp, &format!("{:?}", e.old_state))
                } else {
                    Vec::new()
                }
            },
            DockDoorEvent::SensorStateChanged(e) => {
                if e.sensor_name == "TRAILER_AT_DOOR" && e.new_value == Some(0) {
                    self.generate_undocking_results(door, e.timestamp, "TRAILER_DOCKING")
                } else {
                    Vec::new()
                }
            },
            _ => Vec::new(),
        }
    }
}