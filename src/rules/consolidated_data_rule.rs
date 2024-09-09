use std::sync::Arc;
use dashmap::DashMap;
use chrono::{NaiveDateTime, Duration};
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult};
use crate::models::{DockDoor, DockDoorEvent};
use crate::errors::DockManagerError;
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

const TTL_DURATION: Duration = Duration::hours(24); // Adjust this value as needed

pub struct ConsolidatedDataRule {
    consolidated_events: Arc<DashMap<(String, String, i32), (ConsolidatedDockEvent, NaiveDateTime)>>,
}

impl ConsolidatedDataRule {
    pub fn new() -> Self {
        let consolidated_events = Arc::new(DashMap::new());

        // Spawn a background task to clean up old entries
        let events_clone = Arc::clone(&consolidated_events);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await; // Run every hour
                Self::cleanup_old_entries(&events_clone);
            }
        });

        Self { consolidated_events }
    }

    fn cleanup_old_entries(events: &DashMap<(String, String, i32), (ConsolidatedDockEvent, NaiveDateTime)>) {
        let now = chrono::Local::now().naive_local();
        events.retain(|_, (_, last_updated)| {
            now.signed_duration_since(*last_updated) < TTL_DURATION
        });
    }

    fn update_consolidated_event(&self, door: &DockDoor, event: &DockDoorEvent) -> Result<Option<ConsolidatedDockEvent>, DockManagerError> {
        let (plant_id, door_name, shipment_id, event_timestamp) = match event {
            DockDoorEvent::ShipmentAssigned(e) => (e.plant_id.clone(), e.dock_name.clone(), e.shipment_id.parse::<i32>().unwrap_or(0), e.timestamp),
            DockDoorEvent::DockAssigned(e) => (e.plant_id.clone(), e.dock_name.clone(), e.shipment_id.parse::<i32>().unwrap_or(0), e.timestamp),
            DockDoorEvent::TrailerDocked(e) => (e.plant_id.clone(), e.dock_name.clone(), e.shipment_id.parse::<i32>().unwrap_or(0), e.timestamp),
            DockDoorEvent::ShipmentStarted(e) => (e.base_event.plant_id.clone(), e.base_event.dock_name.clone(), e.base_event.shipment_id.parse::<i32>().unwrap_or(0), e.base_event.timestamp),
            DockDoorEvent::LgvStartLoading(e) => (e.base_event.plant_id.clone(), e.base_event.dock_name.clone(), e.base_event.shipment_id.parse::<i32>().unwrap_or(0), e.base_event.timestamp),
            _ => return Ok(None), // Ignore other event types
        };

        let key = (plant_id.clone(), door_name.clone(), shipment_id);
        let mut entry = self.consolidated_events
            .entry(key.clone())
            .or_insert_with(|| (ConsolidatedDockEvent {
                plant: plant_id,
                door_name: door_name,
                shipment_id,
                docking_time_minutes: None,
                inspection_time_minutes: None,
                enqueued_time_minutes: None,
                shipment_assigned: None,
                dock_assignment: None,
                trailer_docking: None,
                started_shipment: None,
                lgv_start_loading: None,
                dock_ready: None,
                is_preload: door.is_preload,
            }, chrono::Local::now().naive_local()));

        // Update the relevant field based on the event type
        match event {
            DockDoorEvent::ShipmentAssigned(_) => entry.0.shipment_assigned = Some(event_timestamp),
            DockDoorEvent::DockAssigned(_) => entry.0.dock_assignment = Some(event_timestamp),
            DockDoorEvent::TrailerDocked(_) => entry.0.trailer_docking = Some(event_timestamp),
            DockDoorEvent::ShipmentStarted(_) => entry.0.started_shipment = Some(event_timestamp),
            DockDoorEvent::LgvStartLoading(_) => entry.0.lgv_start_loading = Some(event_timestamp),
            _ => {}
        }

        // Update dock_ready time if available
        if entry.0.dock_ready.is_none() {
            entry.0.dock_ready = door.last_dock_ready_time;
        }

        // Update the last updated timestamp
        entry.1 = chrono::Local::now().naive_local();

        self.calculate_durations(&mut entry.0);

        log::info!("Updated consolidated event for shipment {}: {:?}", shipment_id, entry.0);

        // Only return the consolidated event if it's an LgvStartLoading event
        if let DockDoorEvent::LgvStartLoading(_) = event {
            let final_event = entry.0.clone();
            self.consolidated_events.remove(&key);
            Ok(Some(final_event))
        } else {
            Ok(None)
        }
    }

    fn calculate_durations(&self, event: &mut ConsolidatedDockEvent) {
        if let (Some(dock_assignment), Some(trailer_docking)) = (event.dock_assignment.or(event.shipment_assigned), event.trailer_docking) {
            event.docking_time_minutes = Some(Self::calculate_duration_minutes(dock_assignment, trailer_docking));
        }

        if let (Some(trailer_docking), Some(started_shipment)) = (event.trailer_docking.or(event.dock_ready), event.started_shipment) {
            event.inspection_time_minutes = Some(Self::calculate_duration_minutes(trailer_docking, started_shipment));
        }

        if let (Some(started_shipment), Some(lgv_start_loading)) = (event.started_shipment, event.lgv_start_loading) {
            event.enqueued_time_minutes = Some(Self::calculate_duration_minutes(started_shipment, lgv_start_loading));
        }
    }

    fn calculate_duration_minutes(start: NaiveDateTime, end: NaiveDateTime) -> i32 {
        (end - start).num_minutes() as i32
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