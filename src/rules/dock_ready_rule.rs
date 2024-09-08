use crate::models::{DockDoor, DockDoorEvent, DoorState};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult, AlertType, LogEntry};
use chrono::Local;
use log::info;

pub struct DockReadyRule;

impl AnalysisRule for DockReadyRule {
    fn apply(&self, dock_door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        info!("DockReadyRule applying to event: {:?}", event);

        let mut results = Vec::new();

        match event {
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "RH_DOCK_READY" => {
                if e.new_value == Some(1) && e.old_value == Some(0)  {
                    info!("Door ready detected, checking conditions...");

                    results.push(AnalysisResult::Alert(AlertType::DockReady {
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        timestamp: e.timestamp,
                    }));

                    let log_entry = LogEntry::DockingTime {
                        log_dttm: Local::now().naive_local(),
                        plant: dock_door.plant_id.clone(),
                        door_name: dock_door.dock_name.clone(),
                        shipment_id: dock_door.assigned_shipment.current_shipment.clone(),
                        event_type: "DOCK_READY".to_string(),
                        success: true,
                        notes: "Dock ready, docking process completed successfully".to_string(),
                        severity: 0,
                        previous_state: Some(format!("{:?}", DoorState::TrailerDocked)),
                        previous_state_dttm: Some(e.timestamp),
                    };

                    results.push(AnalysisResult::Log(log_entry));
                }
            },
            _ => {}
        }
        info!("DockReadyRule results: {:?}", results);
        results
    }
}