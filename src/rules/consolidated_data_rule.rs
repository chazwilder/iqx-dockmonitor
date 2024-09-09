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
        let shipment_id = event.get_shipment_id().and_then(|id| id.parse::<i32>().ok()).unwrap_or(0);
        let key = (door.plant_id.clone(), door.dock_name.clone(), shipment_id);

        let mut entry = self.consolidated_events
            .entry(key.clone())
            .or_insert_with(|| ConsolidatedDockEvent {
                plant: door.plant_id.clone(),
                door_name: door.dock_name.clone(),
                shipment_id,
                docking_time_minutes: None,
                inspection_time_minutes: None,
                enqueued_time_minutes: None,
                shipment_assigned: None,
                dock_assignment: None,
                trailer_docking: None,
                started_shipment: None,
                lgv_start_loading: None,
                dock_ready: door.dock_assignment,
                is_preload: door.is_preload,
            });

        match event {
            DockDoorEvent::ShipmentAssigned(e) => {
                entry.shipment_assigned = Some(e.timestamp);
                entry.dock_assignment = Some(e.timestamp);
            },
            DockDoorEvent::TrailerDocked(e) => {
                entry.trailer_docking = Some(e.timestamp);
            },
            DockDoorEvent::ShipmentStarted(e) => {
                entry.started_shipment = Some(e.base_event.timestamp);
            },
            DockDoorEvent::LgvStartLoading(e) => {
                entry.lgv_start_loading = Some(e.base_event.timestamp);
            },
            _ => {}
        }

        self.calculate_durations(&mut entry);

        log::info!("Updated consolidated event for shipment {}: {:?}", shipment_id, entry);

        Ok(Some(entry.clone()))
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