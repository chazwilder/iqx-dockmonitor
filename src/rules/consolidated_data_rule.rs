use std::sync::Arc;
use dashmap::DashMap;
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult};
use crate::models::{DockDoor, DockDoorEvent};
use crate::errors::DockManagerError;
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

pub struct ConsolidatedDataRule {
    consolidated_events: Arc<DashMap<(String, String, i32), ConsolidatedDockEvent>>,
}

impl ConsolidatedDataRule {
    pub fn new() -> Self {
        Self {
            consolidated_events: Arc::new(DashMap::new()),
        }
    }

    fn update_consolidated_event(&self, door: &DockDoor, event: &DockDoorEvent) -> Result<Option<ConsolidatedDockEvent>, DockManagerError> {
        match event {
            DockDoorEvent::LgvStartLoading(e) => {
                let shipment_id = e.base_event.shipment_id.parse::<i32>().unwrap_or(0);
                let key = (door.plant_id.clone(), door.dock_name.clone(), shipment_id);

                let mut entry = ConsolidatedDockEvent {
                    plant: door.plant_id.clone(),
                    door_name: door.dock_name.clone(),
                    shipment_id,
                    docking_time_minutes: None,
                    inspection_time_minutes: None,
                    enqueued_time_minutes: None,
                    shipment_assigned: door.assigned_shipment.assignment_dttm,
                    dock_assignment: door.dock_assignment,
                    trailer_docking: door.docking_time,
                    started_shipment: door.shipment_started_dttm,
                    lgv_start_loading: Some(e.base_event.timestamp),
                    dock_ready: door.last_dock_ready_time,
                    is_preload: door.is_preload,
                };

                self.calculate_durations(&mut entry);

                self.consolidated_events.insert(key, entry.clone());

                log::info!("Created/Updated consolidated event for shipment {}: {:?}", shipment_id, entry);

                Ok(Some(entry))
            },
            _ => Ok(None),
        }
    }

    fn calculate_durations(&self, event: &mut ConsolidatedDockEvent) {
        if let (Some(dock_assignment), Some(trailer_docking)) = (event.dock_assignment.or(event.shipment_assigned), event.trailer_docking) {
            event.docking_time_minutes = Some((trailer_docking - dock_assignment).num_minutes() as i32);
        }

        if let (Some(trailer_docking), Some(started_shipment)) = (event.trailer_docking.or(event.dock_ready), event.started_shipment) {
            event.inspection_time_minutes = Some((started_shipment - trailer_docking).num_minutes() as i32);
        }

        if let (Some(started_shipment), Some(lgv_start_loading)) = (event.started_shipment, event.lgv_start_loading) {
            event.enqueued_time_minutes = Some((lgv_start_loading - started_shipment).num_minutes() as i32);
        }
    }
}

impl AnalysisRule for ConsolidatedDataRule {
    fn apply(&self, door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        match self.update_consolidated_event(door, event) {
            Ok(Some(consolidated_event)) => {
                vec![AnalysisResult::ConsolidatedEvent(consolidated_event)]
            },
            Ok(None) => vec![],
            Err(e) => {
                log::error!("Error updating consolidated event: {:?}", e);
                vec![]
            }
        }
    }
}