use std::str::FromStr;
use crate::models::{WmsDoorStatus, DockDoorEvent, DoorState, DockDoor, ShipmentAssignedEvent, ShipmentUnassignedEvent, LoadingStatus, LoadingStatusChangedEvent, DoorStateChangedEvent};
use crate::errors::DockManagerError;
use crate::state_management::door_state_repository::DoorStateRepository;
use std::sync::Arc;

/// Processes WMS (Warehouse Management System) data updates for the dock monitoring system.
pub struct WmsDataProcessor {
    /// Repository for managing dock door states.
    door_repository: Arc<DoorStateRepository>,
}

impl WmsDataProcessor {
    /// Creates a new `WmsDataProcessor`.
    ///
    /// # Arguments
    ///
    /// * `door_repository` - A reference to the `DoorStateRepository` for managing door states.
    ///
    /// # Returns
    ///
    /// A new instance of `WmsDataProcessor`.
    pub fn new(door_repository: Arc<DoorStateRepository>) -> Self {
        Self {
            door_repository,
        }
    }

    /// Processes a batch of WMS data updates and generates corresponding events.
    ///
    /// # Arguments
    ///
    /// * `wms_data` - A vector of `WmsDoorStatus` representing the WMS updates.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of `DockDoorEvent`s generated from the WMS updates,
    /// or a `DockManagerError` if processing fails.
    pub async fn process_wms_updates(&self, wms_data: Vec<WmsDoorStatus>) -> Result<Vec<DockDoorEvent>, DockManagerError> {
        let mut events = Vec::new();

        for wms_status in wms_data {
            let door_events = self.process_single_wms_update(&wms_status).await?;
            events.extend(door_events);
        }

        Ok(events)
    }

    /// Processes a single WMS update for a specific door.
    ///
    /// # Arguments
    ///
    /// * `wms_status` - The `WmsDoorStatus` containing the WMS update for a door.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of `DockDoorEvent`s generated from the WMS update,
    /// or a `DockManagerError` if processing fails.
    async fn process_single_wms_update(&self, wms_status: &WmsDoorStatus) -> Result<Vec<DockDoorEvent>, DockManagerError> {
        let mut events = Vec::new();

        let mut door = self.door_repository.get_door_state(wms_status.plant.as_str(),&wms_status.dock_name)
            .ok_or_else(|| DockManagerError::DoorNotFound(wms_status.dock_name.clone()))?;

        // Update shipment assignment
        if door.assigned_shipment.current_shipment != wms_status.assigned_shipment {
            let old_shipment = door.assigned_shipment.current_shipment.clone();
            door.assigned_shipment.current_shipment = wms_status.assigned_shipment.clone();

            if let Some(shipment_id) = &wms_status.assigned_shipment {
                events.push(DockDoorEvent::ShipmentAssigned(ShipmentAssignedEvent {
                    plant_id: wms_status.plant.clone(),
                    dock_name: door.dock_name.clone(),
                    shipment_id: shipment_id.clone(),
                    timestamp: chrono::Local::now().naive_local(),
                    previous_shipment: old_shipment,
                }));
            } else if let Some(previous_shipment) = old_shipment {
                events.push(DockDoorEvent::ShipmentUnassigned(ShipmentUnassignedEvent {
                    plant_id: wms_status.plant.clone(),
                    dock_name: door.dock_name.clone(),
                    shipment_id: previous_shipment,
                    timestamp: chrono::Local::now().naive_local(),
                }));
            }
        }

        // Update loading status
        let new_loading_status = LoadingStatus::from_str(&wms_status.loading_status)
            .map_err(|_| DockManagerError::ConfigError(format!("Invalid loading status: {}", wms_status.loading_status)))?;

        if door.loading_status != new_loading_status {
            events.push(DockDoorEvent::LoadingStatusChanged(LoadingStatusChangedEvent {
                plant_id: wms_status.plant.clone(),
                dock_name: door.dock_name.clone(),
                old_status: door.loading_status,
                new_status: new_loading_status,
                timestamp: chrono::Local::now().naive_local(),
            }));
            door.loading_status = new_loading_status;
        }

        // Update WMS shipment status
        door.wms_shipment_status = wms_status.wms_shipment_status.clone();

        // Update door state based on WMS data
        let new_door_state = self.determine_door_state(&door, wms_status);
        if door.door_state != new_door_state {
            events.push(DockDoorEvent::DoorStateChanged(DoorStateChangedEvent {
                plant_id: wms_status.plant.clone(),
                dock_name: door.dock_name.clone(),
                old_state: door.door_state,
                new_state: new_door_state,
                timestamp: chrono::Local::now().naive_local(),
            }));
            door.door_state = new_door_state;
        }

        // Update the door in the repository
        self.door_repository.update_door(door.plant_id.clone().as_str(), door)?;

        Ok(events)
    }

    /// Determines the appropriate door state based on WMS data and current door state.
    ///
    /// # Arguments
    ///
    /// * `door` - The current `DockDoor` state.
    /// * `wms_status` - The `WmsDoorStatus` containing the WMS update for the door.
    ///
    /// # Returns
    ///
    /// The new `DoorState` based on the WMS data and current door state.
    fn determine_door_state(&self, door: &DockDoor, wms_status: &WmsDoorStatus) -> DoorState {
        match wms_status.loading_status.as_str() {
            "Idle" => DoorState::Unassigned,
            "CSO" => DoorState::Assigned,
            "WhseInspection" => DoorState::DriverCheckedIn,
            "LgvAllocation" => DoorState::DoorReady,
            "Loading" => DoorState::Loading,
            "Completed" => DoorState::LoadingCompleted,
            "WaitingForExit" => DoorState::WaitingForExit,
            _ => door.door_state, // Maintain current state if unknown WMS status
        }
    }
}