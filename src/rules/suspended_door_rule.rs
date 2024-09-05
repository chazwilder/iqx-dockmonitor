use std::collections::HashMap;
use chrono::{NaiveDateTime, Local, Duration};
use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use crate::models::{DockDoor, DockDoorEvent};

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
            DockDoorEvent::WmsEvent(e) if e.event_type == "SUSPENDED_SHIPMENT" => {
                if self.should_send_alert(&dock_door.dock_name) {
                    let duration = e.timestamp.signed_duration_since(
                        dock_door.assigned_shipment.assignment_dttm.unwrap_or(e.timestamp)
                    );
                    let user = e.message_notes
                        .as_ref()
                        .and_then(|notes| notes.split('-').next())
                        .map(|user| user.trim().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    results.push(AnalysisResult::Alert(AlertType::SuspendedDoor {
                        door_name: dock_door.dock_name.clone(),
                        duration,
                        shipment_id: Some(e.shipment_id.clone()),
                        user,
                    }));

                    // Update the log entry as well
                    if let Some(AnalysisResult::Alert(AlertType::SuspendedDoor { door_name, duration, shipment_id, user })) = results.last() {
                        let log_entry = LogEntry::SuspendedDoor {
                            log_dttm: Local::now().naive_local(),
                            plant: dock_door.plant_id.clone(),
                            door_name: door_name.clone(),
                            shipment_id: shipment_id.clone(),
                            event_type: "SUSPENDED_DOOR_WMS".to_string(),
                            success: false,
                            notes: format!("Door suspended for {} by user {}", self.format_duration(duration), user),
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