use crate::models::{DockDoor, DockDoorEvent, TrailerState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::analysis::LogEntry;

/// Configuration for the `NewShipmentPreviousTrailerPresentRule`
#[derive(Debug, Deserialize, Serialize)]
pub struct NewShipmentPreviousTrailerPresentRuleConfig {
    /// The list of WMS shipment statuses that are considered "complete" for the purpose of this rule
    pub completion_statuses: Vec<String>,
}

/// An analysis rule that detects and alerts when a new shipment is assigned to a dock door while the previous trailer is still present
pub struct NewShipmentPreviousTrailerPresentRule {
    /// The configuration for this rule
    config: NewShipmentPreviousTrailerPresentRuleConfig,
}

impl NewShipmentPreviousTrailerPresentRule {
    /// Creates a new `NewShipmentPreviousTrailerPresentRule` with the given configuration
    pub fn new(config: Value) -> Self {
        let parsed_config: NewShipmentPreviousTrailerPresentRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse NewShipmentPreviousTrailerPresentRule configuration");
        NewShipmentPreviousTrailerPresentRule { config: parsed_config }
    }

    /// Checks if the previous shipment associated with the dock door is considered complete
    ///
    /// The method looks at the `wms_shipment_status` of the `dock_door` and checks if it's present in the `completion_statuses`
    /// defined in the rule's configuration
    ///
    /// # Arguments
    ///
    /// * `dock_door`: A reference to the `DockDoor` object
    ///
    /// # Returns
    ///
    /// * `true` if the previous shipment is complete, `false` otherwise
    fn is_previous_shipment_complete(&self, dock_door: &DockDoor) -> bool {
        if let Some(status) = &dock_door.wms_shipment_status {
            self.config.completion_statuses.contains(status)
        } else {
            false
        }
    }
}

impl AnalysisRule for NewShipmentPreviousTrailerPresentRule {
    /// Applies the rule to a dock door event, generating an alert and a log entry if a new shipment is assigned while the previous trailer is still present
    ///
    /// The method checks if the event is a `ShipmentAssignedEvent`. If so, it further checks if:
    /// 1. The trailer is currently docked (`TrailerState::Docked`)
    /// 2. The previous shipment is considered complete (using `is_previous_shipment_complete`)
    ///
    /// If both conditions are met, it generates:
    /// - An `AlertType::NewShipmentPreviousTrailerPresent` alert
    /// - A `LogEntry::NewShipmentPreviousTrailerPresent` log entry
    ///
    /// These are wrapped in `AnalysisResult` and returned in a vector
    /// If the event is not a shipment assignment or the conditions are not met, an empty vector is returned
    ///
    /// # Arguments
    ///
    /// * `dock_door`: A reference to the `DockDoor` object the event is associated with
    /// * `event`: A reference to the `DockDoorEvent` to be analyzed
    ///
    /// # Returns
    ///
    /// A vector containing an alert and a log entry if the rule conditions are met, otherwise an empty vector
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match event {
            DockDoorEvent::ShipmentAssigned(e) => {
                if dock_door.trailer_state == TrailerState::Docked && self.is_previous_shipment_complete(dock_door) {
                    let log_entry = LogEntry::NewShipmentPreviousTrailerPresent {
                        log_dttm: e.timestamp,
                        plant: dock_door.plant_id.clone(),
                        door_name: e.dock_name.clone(),
                        shipment_id: Some(e.shipment_id.clone()),
                        event_type: "NEW_SHIPMENT_PREVIOUS_TRAILER_PRESENT".to_string(),
                        success: false, // This is generally considered an issue, so marking as not successful
                        notes: format!("New shipment {} assigned while previous trailer (shipment: {:?}) is still present",
                                       e.shipment_id, e.previous_shipment),
                        severity: 2, // Considering this a moderate severity issue
                        previous_state: Some("PREVIOUS_SHIPMENT_DOCKED".to_string()),
                        previous_state_dttm: dock_door.trailer_state_changed,
                    };

                    vec![
                        AnalysisResult::Alert(AlertType::NewShipmentPreviousTrailerPresent {
                            dock_name: e.dock_name.clone(),
                            new_shipment: e.shipment_id.clone(),
                            previous_shipment: e.previous_shipment.clone(),
                            timestamp: e.timestamp,
                        }),
                        AnalysisResult::Log(log_entry)
                    ]
                } else {
                    vec![]
                }
            },
            _ => vec![],
        }
    }
}