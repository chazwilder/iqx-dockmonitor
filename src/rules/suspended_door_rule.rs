use std::collections::HashMap;
use chrono::{NaiveDateTime, Local, Duration};
use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use crate::models::{DockDoor, DockDoorEvent, LoadingStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendedDoorRuleConfig {
    pub alert_threshold: u64,
    pub repeat_interval: u64,
}

pub struct SuspendedDoorRule {
    config: SuspendedDoorRuleConfig,
    last_alert_time: HashMap<String, NaiveDateTime>,
}

impl SuspendedDoorRule {
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: SuspendedDoorRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse SuspendedDoorRule configuration");
        Self {
            config: parsed_config,
            last_alert_time: HashMap::new(),
        }
    }

    fn should_send_alert(&self, door_name: &str) -> bool {
        let now = Local::now().naive_local();
        let last_alert = self.last_alert_time.get(door_name);

        match last_alert {
            Some(time) if now.signed_duration_since(*time) < Duration::seconds(self.config.repeat_interval as i64) => false,
            _ => {
                self.last_alert_time.clone().insert(door_name.to_string(), now);
                true
            }
        }
    }

    fn format_duration(&self, duration: &Duration) -> String {
        let total_seconds = duration.num_seconds();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

impl AnalysisRule for SuspendedDoorRule {
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let mut results = Vec::new();

        match event {
            DockDoorEvent::LoadingStatusChanged(e) if e.new_status == LoadingStatus::Suspended => {
                let suspension_duration = Local::now().naive_local().signed_duration_since(e.timestamp);

                if suspension_duration > Duration::seconds(self.config.alert_threshold as i64) {
                    if self.should_send_alert(&dock_door.dock_name) {
                        results.push(AnalysisResult::Alert(AlertType::SuspendedDoor {
                            door_name: dock_door.dock_name.clone(),
                            duration: suspension_duration,
                            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        }));

                        // NEW CODE: Add log entry for the suspended door
                        if let Some(AnalysisResult::Alert(AlertType::SuspendedDoor { door_name, duration, shipment_id })) = results.last() {
                            let log_entry = LogEntry::SuspendedDoor {
                                log_dttm: Local::now().naive_local(),
                                plant: dock_door.plant_id.clone(),
                                door_name: door_name.clone(),
                                shipment_id: shipment_id.clone(),
                                event_type: "SUSPENDED_DOOR".to_string(),
                                success: false,
                                notes: format!("Door suspended for {}", self.format_duration(duration)),
                                severity: 2,
                                previous_state: Some(format!("{:?}", e.old_status)),
                                previous_state_dttm: Some(e.timestamp),
                            };
                            results.push(AnalysisResult::Log(log_entry));
                        }
                    }
                }
            },
            DockDoorEvent::WmsEvent(e) if e.event_type == "SUSPENDED_SHIPMENT" => {
                if self.should_send_alert(&dock_door.dock_name) {
                    let duration = e.timestamp.signed_duration_since(
                        dock_door.assigned_shipment.assignment_dttm.unwrap_or(e.timestamp)
                    );
                    results.push(AnalysisResult::Alert(AlertType::SuspendedDoor {
                        door_name: dock_door.dock_name.clone(),
                        duration,
                        shipment_id: Some(e.shipment_id.clone()),
                    }));

                    // NEW CODE: Add log entry for the suspended door (WMS event)
                    if let Some(AnalysisResult::Alert(AlertType::SuspendedDoor { door_name, duration, shipment_id })) = results.last() {
                        let log_entry = LogEntry::SuspendedDoor {
                            log_dttm: Local::now().naive_local(),
                            plant: dock_door.plant_id.clone(),
                            door_name: door_name.clone(),
                            shipment_id: shipment_id.clone(),
                            event_type: "SUSPENDED_DOOR_WMS".to_string(),
                            success: false,
                            notes: format!("Door suspended for {} (WMS event)", self.format_duration(duration)),
                            severity: 2,
                            previous_state: None,
                            previous_state_dttm: None,
                        };
                        results.push(AnalysisResult::Log(log_entry));
                    }
                }
            },
            _ => {}
        }

        results
    }
}