use serde::{Deserialize, Serialize};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType};
use crate::models::{DockDoor, DockDoorEvent, DockLockState, DoorPosition, LevelerPosition};

/// Configuration for the ShipmentStartedLoadNotReadyRule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentStartedLoadNotReadyRuleConfig {
    /// Whether to check if the dock restraint is engaged
    pub check_restraint: bool,
    /// Whether to check if the dock leveler is extended
    pub check_leveler: bool,
    /// Whether to check if the dock door is open
    pub check_door_open: bool,
}

/// Rule for detecting when a shipment has started loading but the dock is not ready
pub struct ShipmentStartedLoadNotReadyRule {
    /// The parsed configuration for this rule
    config: ShipmentStartedLoadNotReadyRuleConfig,
}

impl ShipmentStartedLoadNotReadyRule {
    /// Creates a new ShipmentStartedLoadNotReadyRule with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The JSON configuration containing the rule parameters
    ///
    /// # Returns
    ///
    /// A new instance of ShipmentStartedLoadNotReadyRule
    pub fn new(config: serde_json::Value) -> Self {
        let parsed_config: ShipmentStartedLoadNotReadyRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse ShipmentStartedLoadNotReadyRule configuration");
        Self { config: parsed_config }
    }

    /// Checks if the dock is ready for loading based on the rule configuration
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to check
    ///
    /// # Returns
    ///
    /// A vector of reasons why the dock is not ready, if any
    fn check_dock_readiness(&self, dock_door: &DockDoor) -> Vec<String> {
        let mut reasons = Vec::new();

        if self.config.check_restraint && dock_door.dock_lock_state != DockLockState::Engaged {
            reasons.push("Dock restraint is not engaged".to_string());
        }

        if self.config.check_leveler && dock_door.leveler_position != LevelerPosition::Extended {
            reasons.push("Dock leveler is not extended".to_string());
        }

        if self.config.check_door_open && dock_door.door_position != DoorPosition::Open {
            reasons.push("Dock door is not open".to_string());
        }

        reasons
    }
}

impl AnalysisRule for ShipmentStartedLoadNotReadyRule {
    /// Applies the ShipmentStartedLoadNotReadyRule to the given dock door and event
    ///
    /// # Arguments
    ///
    /// * `dock_door` - The DockDoor to which the rule is being applied
    /// * `event` - The DockDoorEvent being processed
    ///
    /// # Returns
    ///
    /// A vector of AnalysisResult, which may contain alerts if the rule conditions are met
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let mut results = Vec::new();

        match event {
            DockDoorEvent::WmsEvent(e) if e.event_type == "STARTED_SHIPMENT" => {
                let reasons = self.check_dock_readiness(dock_door);
                if !reasons.is_empty() {
                    results.push(AnalysisResult::Alert(AlertType::ShipmentStartedLoadNotReady {
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: e.shipment_id.clone(),
                        reason: reasons.join(", "),
                    }));
                }
            },
            _ => {}
        }

        results
    }
}