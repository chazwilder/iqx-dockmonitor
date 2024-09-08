use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use crate::models::{DockDoor, DockDoorEvent};
use chrono::Local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailerPatternRuleConfig {
    pub severity_threshold: i32,
}

pub struct TrailerPatternRule {
    config: TrailerPatternRuleConfig,
}

impl TrailerPatternRule {
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: TrailerPatternRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse TrailerPatternRule configuration");
        Self { config: parsed_config }
    }

    fn parse_trl_ptn_value(&self, message_notes: &str) -> Option<i32> {
        message_notes.parse::<i32>().ok()
    }
}

impl AnalysisRule for TrailerPatternRule {
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let mut results = Vec::new();
        match event {
            DockDoorEvent::WmsEvent(e) if e.event_type == "TRK_PTRN" => {
                if let Some(message_notes) = &e.message_notes {
                    if let Some(pattern_value) = self.parse_trl_ptn_value(message_notes) {
                        if pattern_value > 0 && pattern_value > self.config.severity_threshold {
                            results.push(AnalysisResult::Alert(AlertType::TrailerPatternIssue {
                                door_name: dock_door.dock_name.clone(),
                                issue: format!("Trailer pattern issue detected: {}", pattern_value),
                                severity: pattern_value,
                                shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                            }));

                            if let Some(AnalysisResult::Alert(AlertType::TrailerPatternIssue { door_name, issue, severity, shipment_id })) = results.last() {
                                let log_entry = LogEntry::TrailerPatternIssue {
                                    log_dttm: Local::now().naive_local(),
                                    plant: dock_door.plant_id.clone(),
                                    door_name: door_name.clone(),
                                    shipment_id: shipment_id.clone(),
                                    event_type: "TRAILER_PATTERN_ISSUE".to_string(),
                                    success: false,
                                    notes: issue.clone(),
                                    severity: *severity,
                                    previous_state: None,
                                    previous_state_dttm: None,
                                };
                                results.push(AnalysisResult::Log(log_entry));
                            }
                        }
                    }
                }
            },
            _ => {}
        }
        results
    }
}