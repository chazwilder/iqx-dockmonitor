use crate::models::{DockDoor, DockDoorEvent, LoadingStatus, TrailerState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, LogEntry, AlertType};
use chrono::Local;
use tracing::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct TrailerDockingRuleConfig {
    pub sensors_to_monitor: Vec<SensorConfig>,
    pub fields_to_monitor: Vec<String>,
    pub successful_dock_conditions: SuccessfulDockConditions,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SensorConfig {
    pub name: String,
    pub success_value: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SuccessfulDockConditions {
    pub loading_status: Vec<String>,
    pub wms_shipment_status: Vec<String>,
}

pub struct TrailerDockingRule {
    config: TrailerDockingRuleConfig,
}

impl TrailerDockingRule {
    pub fn new(config: Value) -> Self {
        let parsed_config: TrailerDockingRuleConfig = serde_json::from_value(config)
            .expect("Failed to parse TrailerDockingRule configuration");
        TrailerDockingRule { config: parsed_config }
    }

    fn is_docking_successful(&self, dock_door: &DockDoor) -> bool {
        let shipment_condition = dock_door.assigned_shipment.current_shipment.is_some();
        if !shipment_condition {
            return false;
        }

        let sensor_conditions = self.check_sensor_conditions(dock_door);
        let loading_status_condition = self.check_loading_status(dock_door);

        info!(
            "DockDoor: {} - Docking conditions: sensors={}, loading_status={}, shipment={}",
            dock_door.dock_name,
            sensor_conditions,
            loading_status_condition,
            shipment_condition
        );

        sensor_conditions && loading_status_condition && shipment_condition
    }

    fn check_sensor_conditions(&self, dock_door: &DockDoor) -> bool {
        let trailer_angle = dock_door.sensors.get("TRAILER_ANGLE")
            .map(|sensor| sensor.get_sensor_data().current_value == Some(0))
            .unwrap_or(false);
        let trailer_centering = dock_door.sensors.get("TRAILER_CENTERING")
            .map(|sensor| sensor.get_sensor_data().current_value == Some(0))
            .unwrap_or(false);
        let trailer_distance = dock_door.sensors.get("TRAILER_DISTANCE")
            .map(|sensor| sensor.get_sensor_data().current_value == Some(0))
            .unwrap_or(false);
        let trailer_at_door = dock_door.sensors.get("TRAILER_AT_DOOR")
            .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
            .unwrap_or(false);

        info!(
            "DockDoor: {} - Sensor conditions: angle={}, centering={}, distance={}, at_door={}",
            dock_door.dock_name,
            trailer_angle,
            trailer_centering,
            trailer_distance,
            trailer_at_door
        );

        trailer_at_door && trailer_angle && trailer_centering && trailer_distance
    }

    fn check_loading_status(&self, dock_door: &DockDoor) -> bool {
        match (&dock_door.loading_status, &dock_door.wms_shipment_status) {
            (LoadingStatus::CSO, _) | (LoadingStatus::WhseInspection, _) => true,
            (_, Some(status)) => {
                self.config.successful_dock_conditions.wms_shipment_status.contains(&status)
            },
            _ => false,
        }
    }

    fn check_manual_mode_alert(&self, dock_door: &DockDoor) -> Option<AlertType> {
        let trailer_at_door = dock_door.sensors.get("TRAILER_AT_DOOR")
            .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
            .unwrap_or(false);

        let manual_mode = dock_door.sensors.get("RH_MANUAL_MODE")
            .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
            .unwrap_or(false);

        let other_sensors_ok = ["TRAILER_ANGLE", "TRAILER_CENTERING", "TRAILER_DISTANCE"]
            .iter()
            .all(|&sensor_name| {
                dock_door.sensors.get(sensor_name)
                    .map(|sensor| sensor.get_sensor_data().current_value == Some(0))
                    .unwrap_or(false)
            });

        if trailer_at_door && manual_mode && other_sensors_ok {
            Some(AlertType::ManualModeAlert)
        } else {
            None
        }
    }

    fn get_failure_reason(&self, dock_door: &DockDoor) -> String {
        let mut reasons = Vec::new();

        if !self.check_sensor_conditions(dock_door) {
            if !dock_door.sensors.get("TRAILER_AT_DOOR")
                .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
                .unwrap_or(false) {
                reasons.push("Trailer not at door");
            }
            if dock_door.sensors.get("TRAILER_ANGLE")
                .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
                .unwrap_or(false) {
                reasons.push("Trailer angle issue");
            }
            if dock_door.sensors.get("TRAILER_CENTERING")
                .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
                .unwrap_or(false) {
                reasons.push("Trailer centering issue");
            }
            if dock_door.sensors.get("TRAILER_DISTANCE")
                .map(|sensor| sensor.get_sensor_data().current_value == Some(1))
                .unwrap_or(false) {
                reasons.push("Trailer distance issue");
            }
        }

        if !self.check_loading_status(dock_door) {
            reasons.push("Incorrect loading status");
        }

        if dock_door.assigned_shipment.current_shipment.is_none() {
            reasons.push("No shipment assigned");
        }

        if reasons.is_empty() {
            "Unknown docking failure".to_string()
        } else {
            reasons.join(", ")
        }
    }
}

impl AnalysisRule for TrailerDockingRule {
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let mut results = Vec::new();
        match event {
            DockDoorEvent::TrailerStateChanged(e) => {
                if e.new_state == TrailerState::Docked && e.old_state == TrailerState::Undocked {
                    let is_successful = self.is_docking_successful(dock_door);
                    let log_entry = LogEntry::DockingTime {
                        log_dttm: Local::now().naive_local(),
                        plant: dock_door.plant_id.clone(),
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        event_type: "TRAILER_DOCKING".to_string(),
                        success: is_successful,
                        notes: if is_successful {
                            "Trailer docked successfully".to_string()
                        } else {
                            format!("Trailer docking failed: {}", self.get_failure_reason(dock_door))
                        },
                        severity: if is_successful { 0 } else { 2 },
                        previous_state: Some(format!("{:?}", e.old_state)),
                        previous_state_dttm: Some(e.timestamp),
                    };

                    info!("TrailerDockingRule: Generated docking log entry: {:?}", log_entry);
                    results.push(AnalysisResult::Log(log_entry));

                    if let Some(alert) = self.check_manual_mode_alert(dock_door) {
                        info!("TrailerDockingRule: Generated manual mode alert");
                        results.push(AnalysisResult::Alert(alert));
                    }
                }
            },
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "TRAILER_AT_DOOR" => {
                if e.new_value == Some(1) {
                    if matches!(dock_door.loading_status,
                    LoadingStatus::Completed | LoadingStatus::WaitingForExit) {
                        info!("TrailerDockingRule: Ignoring event for completed or waiting-for-exit load");
                        return vec![];
                    }
                    let is_successful = self.is_docking_successful(dock_door);
                    info!("TrailerDockingRule: Docking successful: {}", is_successful);

                    if dock_door.trailer_state == TrailerState::Undocked {
                        let log_entry = LogEntry::DockingTime {
                            log_dttm: Local::now().naive_local(),
                            plant: dock_door.plant_id.clone(),
                            door_name: dock_door.dock_name.clone(),
                            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                            event_type: "TRAILER_DOCKING".to_string(),
                            success: is_successful,
                            notes: if is_successful {
                                "Trailer at door, docking successful".to_string()
                            } else {
                                format!("Trailer at door, docking failed: {}", self.get_failure_reason(dock_door))
                            },
                            severity: if is_successful { 0 } else { 2 },
                            previous_state: Some("TRAILER_UNDOCKING".to_string()),
                            previous_state_dttm: Some(e.timestamp),
                        };

                        info!("TrailerDockingRule: Generated docking log entry: {:?}", log_entry);
                        results.push(AnalysisResult::Log(log_entry));

                        if let Some(alert) = self.check_manual_mode_alert(dock_door) {
                            info!("TrailerDockingRule: Generated manual mode alert");
                            results.push(AnalysisResult::Alert(alert));
                        }
                    }
                }
            },
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "RH_DOCK_READY" => {
                if let Some(sensor) = dock_door.sensors.get(&e.sensor_name) {
                    let previous_value = sensor.get_sensor_data().previous_value;
                    let current_value = sensor.get_sensor_data().current_value;

                    if previous_value == Some(0) && current_value == Some(1) && self.is_docking_successful(dock_door) {
                        let log_entry = LogEntry::DockingTime {
                            log_dttm: Local::now().naive_local(),
                            plant: dock_door.plant_id.clone(),
                            door_name: dock_door.dock_name.clone(),
                            shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                            event_type: "DOCK_READY".to_string(),
                            success: true,
                            notes: "Dock ready, docking process completed successfully".to_string(),
                            severity: 0,
                            previous_state: Some("DOCK_NOT_READY".to_string()),
                            previous_state_dttm: Some(e.timestamp),
                        };
                        info!("TrailerDockingRule: Generated successful docking log entry based on RH_DOCK_READY transition: {:?}", log_entry);
                        results.push(AnalysisResult::Log(log_entry));
                    }
                }
            },
            _ => {},
        }
        results
    }
}