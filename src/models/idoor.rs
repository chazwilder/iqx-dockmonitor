//! # Dock Door Representation

//! This module defines the `DockDoor` struct, which represents the state and data associated with a single dock door. 
//! The `DockDoor` struct encapsulates various attributes such as the door's operational state, loading status, 
//! associated sensors, and shipment information. It also provides methods to handle events and update its state based on 
//! sensor readings and interactions with the Warehouse Management System (WMS).

use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::str::FromStr;
use log::info;
use crate::config::PlantSettings;
use crate::models::isensor::DockSensor;
use crate::models::istates::{DoorState, TrailerState, ManualMode, DockLockState, DoorPosition, LevelerPosition, FaultState};
use crate::models::istatus::LoadingStatus;
use crate::models::ievents::{DockAssignedEvent, DockDoorEvent, DockUnassignedEvent, DoorStateChangedEvent, LoadingCompletedEvent, LoadingStartedEvent, LoadingStatusChangedEvent, SensorStateChangedEvent, TrailerDepartedEvent, TrailerDockedEvent};
use crate::errors::{DockManagerError, DockManagerResult};
use crate::models::{AssignedShipment, RestraintState, ShipmentAssignedEvent, ShipmentUnassignedEvent, TrailerPositionState, TrailerStateChangedEvent, WmsDoorStatus};


/// Represents the result of evaluating a sensor update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorEvaluation {
    /// Indicates whether the sensor value has changed
    pub changed: bool,
    /// The old sensor value before the update
    pub old_value: Option<u8>,
    /// The new sensor value after the update
    pub new_value: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadStatusState {
    /// The current loading status of the door (e.g., Idle, Loading, Completed).
    pub loading_status: LoadingStatus,
    pub current_state_dttm: Option<NaiveDateTime>,
    /// The previous loading status of the door.
    pub previous_loading_status: LoadingStatus,
    pub previous_state_dttm: Option<NaiveDateTime>,
    /// The shipment status as reported by the WMS.
    pub wms_shipment_status: Option<String>,
    pub last_polling_dttm: Option<NaiveDateTime>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsolidatedDataState {
    pub docking_time: Option<NaiveDateTime>,
    pub is_preload: bool,
    pub last_dock_ready_time: Option<NaiveDateTime>,
    pub dock_assignment: Option<NaiveDateTime>,
    pub shipment_started_dttm: Option<NaiveDateTime>,
    pub lgv_loading_started: Option<NaiveDateTime>,
    pub lgv_first_drop: Option<NaiveDateTime>,
}

/// Represents the state and data associated with a single dock door.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockDoor {
    /// The ID of the plant where the dock door is located.
    pub plant_id: String,
    /// The name or identifier of the dock door.
    pub dock_name: String,
    /// The IP address of the PLC controlling the dock door.
    pub dock_ip: String,
    /// The current operational state of the door (e.g., Unassigned, TrailerDocked, Loading).
    pub door_state: DoorState,
    /// The previous operational state of the door
    pub previous_door_state: DoorState,
    /// The current loading status of the door (e.g., Idle, Loading, Completed).
    pub loading_status: LoadStatusState,
    /// The current state of the trailer at the door (Docked or Undocked).
    pub trailer_state: TrailerState,
    /// The previous state of the trailer at the door
    pub previous_trailer_state: TrailerState,
    /// The timestamp when the trailer state last changed
    pub trailer_state_changed: Option<NaiveDateTime>,
    /// Whether the door is currently in manual mode.
    pub manual_mode: ManualMode,
    /// The state of the dock lock (Engaged or Disengaged).
    pub dock_lock_state: DockLockState,
    /// The current position of the door (Open or Closed).
    pub door_position: DoorPosition,
    /// The current position of the leveler (Stored or Extended).
    pub leveler_position: LevelerPosition,
    /// The current fault state of the door (NoFault or FaultPresent).
    pub fault_state: FaultState,
    /// Information about the shipment currently or previously assigned to the door
    pub assigned_shipment: AssignedShipment,
    /// A map of sensor names to their corresponding `DockSensor` objects, representing the sensors associated with the door
    pub sensors: HashMap<String, DockSensor>,
    /// The timestamp when the door's state was last updated
    pub last_updated: NaiveDateTime,
    /// Indicates if there's a fault with the trailer doors
    pub trailer_door_fault: bool,
    /// Indicates if there's a fault with the dock lock
    pub dock_lock_fault: bool,
    /// Indicates if there's a fault with the door
    pub door_fault: bool,
    /// Indicates if the emergency stop has been activated
    pub emergency_stop: bool,
    /// Indicates if there's a fault with the leveler
    pub leveler_fault: bool,
    /// The current state of the restraint (Locking, Unlocking, Locked, Unlocked)
    pub restraint_state: RestraintState,
    /// The current position state of the trailer (Proper or Improper)
    pub trailer_position_state: TrailerPositionState,
    pub consolidated: ConsolidatedDataState,

}

impl DockDoor {
    /// Creates a new `DockDoor` instance
    ///
    /// Initializes a `DockDoor` with the provided plant ID, dock name, and dock IP.
    /// It also sets up the sensors associated with the door based on the plant settings
    /// and initializes other state variables to their default values
    ///
    /// # Arguments
    ///
    /// * `plant_id`: The ID of the plant where the door is located
    /// * `dock_name`: The name or identifier of the dock door
    /// * `dock_ip`: The IP address of the PLC controlling the door
    /// * `plant_settings`: Configuration settings for the plant, including sensor details
    ///
    /// # Returns:
    /// A new `DockDoor` instance
    pub fn new(plant_id: String, dock_name: String, dock_ip: String, plant_settings: &PlantSettings) -> Self {
        let loading_status = LoadStatusState {
            loading_status: LoadingStatus::Idle,
            current_state_dttm: None,
            previous_loading_status: LoadingStatus::Idle,
            wms_shipment_status: None,
            previous_state_dttm: None,
            last_polling_dttm: None
        };
        let consolidated = ConsolidatedDataState{
            docking_time: None,
            is_preload: false,
            last_dock_ready_time: None,
            dock_assignment: None,
            shipment_started_dttm: None,
            lgv_loading_started: None,
            lgv_first_drop: None,
        };
        let mut door = DockDoor {
            plant_id,
            dock_name: dock_name.clone(),
            dock_ip: dock_ip.clone(),
            loading_status,
            door_state: DoorState::Unassigned,
            previous_door_state: DoorState::Unassigned,
            trailer_state: TrailerState::Undocked,
            previous_trailer_state: TrailerState::Undocked,
            trailer_state_changed: None,
            manual_mode: ManualMode::Disabled,
            dock_lock_state: DockLockState::Disengaged,
            door_position: DoorPosition::Closed,
            leveler_position: LevelerPosition::Stored,
            fault_state: FaultState::NoFault,
            assigned_shipment: AssignedShipment::default(),
            sensors: HashMap::new(),
            last_updated: chrono::Local::now().naive_local(),
            trailer_door_fault: false,
            dock_lock_fault: false,
            door_fault: false,
            emergency_stop: false,
            leveler_fault: false,
            restraint_state: RestraintState::Unlocked,
            trailer_position_state: TrailerPositionState::Improper,
            consolidated,
        };
        for tag in &plant_settings.dock_doors.dock_plc_tags {
            door.sensors.insert(
                tag.tag_name.clone(),
                DockSensor::new(
                    &dock_name,
                    &dock_ip,
                    &tag.tag_name,
                    &tag.address,
                )
            );
        }

        door
    }

/// Updates the value of a sensor and evaluates if a change occurred
    ///
    /// This method attempts to update the value of the specified sensor with the new value
    /// It returns a `SensorEvaluation` indicating whether the value actually changed
    /// and providing the old and new values for reference
    ///
    /// If the sensor is not found or the new value is `None`, an error is returned
    ///
    /// # Arguments
    ///
    /// * `sensor_name`: The name of the sensor to update
    /// * `new_value`: The new value to set for the sensor (or `None` if the read failed)
    ///
    /// # Returns
    ///
    /// * `Ok(SensorEvaluation)` if the sensor was found and updated successfully
    /// * `Err(DockManagerError)` if the sensor was not found or the new value is `None`
    pub fn update_sensor(&mut self, sensor_name: &str, new_value: Option<u8>) -> Result<SensorEvaluation, DockManagerError> {
        if let Some(sensor) = self.sensors.get_mut(sensor_name) {
            let old_value = sensor.get_sensor_data().current_value;

            match new_value {
                Some(value) => {
                    if old_value != Some(value) {
                        sensor.update_value(Some(value));
                        Ok(SensorEvaluation { changed: true, old_value, new_value })
                    } else {
                        Ok(SensorEvaluation { changed: false, old_value, new_value })
                    }
                },
                None => {
                    Err(DockManagerError::PlcError(format!("Failed to read sensor {} for door {}", sensor_name, self.dock_name)))
                }
            }
        } else {
            Err(DockManagerError::SensorReadError(sensor_name.to_string()))
        }
    }

    /// Handles an incoming `DockDoorEvent`, updating the door's state accordingly
    ///
    /// This method dispatches the event to the appropriate handler function based on its type
    /// Each handler function is responsible for updating the relevant state variables of the `DockDoor`
    ///
    /// # Arguments
    /// * `event`: The `DockDoorEvent` to be handled
    ///
    /// # Returns
    /// * `Ok(())` if the event was handled successfully
    /// * `Err(DockManagerError)` if an error occurred during event handling
    pub fn handle_event(&mut self, event: &DockDoorEvent) -> Result<(), DockManagerError> {
        match event {
            DockDoorEvent::DockAssigned(e) => self.handle_dock_assigned(e),
            DockDoorEvent::DockUnassigned(e) => self.handle_dock_unassigned(e),
            DockDoorEvent::TrailerDocked(e) => self.handle_trailer_docked(e),
            DockDoorEvent::TrailerDeparted(e) => self.handle_trailer_departed(e),
            DockDoorEvent::LoadingStarted(e) => self.handle_loading_started(e),
            DockDoorEvent::LoadingCompleted(e) => self.handle_loading_completed(e),
            DockDoorEvent::SensorStateChanged(e) => self.handle_sensor_state_changed(e),
            DockDoorEvent::DoorStateChanged(e) => self.handle_door_state_changed(e),
            DockDoorEvent::LoadingStatusChanged(e) => self.handle_loading_status_changed(e),
            DockDoorEvent::TrailerStateChanged(e) => self.handle_trailer_state_changed(e),
            // Add handlers for other events as needed
            _ => Ok(()),
        }
    }

    /// Handles a `DockAssignedEvent`, updating the door's state and shipment information
    ///
    /// If the assigned shipment in the event is different from the current one, 
    /// the door's state is set to `Assigned`, the shipment information is updated, 
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `DockAssignedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_dock_assigned(&mut self, event: &DockAssignedEvent) -> Result<(), DockManagerError> {
        let state_shipment = self.assigned_shipment.current_shipment.clone();
        let event_shipment = Some(event.shipment_id.clone());
        if state_shipment != event_shipment {
            self.door_state = DoorState::Assigned;
            self.assigned_shipment.previous_shipment = self.assigned_shipment.current_shipment.clone();
            self.assigned_shipment.current_shipment = Some(event.shipment_id.clone());
            self.assigned_shipment.assignment_dttm = Some(event.timestamp);
            self.last_updated = event.timestamp;
            return Ok(());
        }
        Ok(())
    }

    /// Handles a `DockUnassignedEvent`, updating the door's state and shipment information
    ///
    /// The door's state is set to `Unassigned`, the current shipment is cleared, 
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `DockUnassignedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_dock_unassigned(&mut self, event: &DockUnassignedEvent) -> Result<(), DockManagerError> {
        self.door_state = DoorState::Unassigned;
        self.assigned_shipment.previous_shipment = self.assigned_shipment.current_shipment.clone();
        self.assigned_shipment.current_shipment = None;
        self.assigned_shipment.assignment_dttm = Some(event.timestamp);
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `TrailerDockedEvent`, updating the door and trailer states
    ///
    /// The trailer state is set to `Docked`, the door state is set to `TrailerDocked`,
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `TrailerDockedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_trailer_docked(&mut self, event: &TrailerDockedEvent) -> Result<(), DockManagerError> {
        info!("TrailerDockedEvent: {:?}", event);
        self.trailer_state = TrailerState::Docked;
        self.door_state = DoorState::TrailerDocked;
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `TrailerDepartedEvent`, updating the door and trailer states
    ///
    /// The trailer state is set to `Undocked`, the door state is set to `WaitingForExit`
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `TrailerDepartedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_trailer_departed(&mut self, event: &TrailerDepartedEvent) -> Result<(), DockManagerError> {
        info!("TrailerDepartedEvent: {:?}", event);

        self.trailer_state = TrailerState::Undocked;
        self.door_state = DoorState::WaitingForExit;
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `LoadingStartedEvent`, updating the loading status and door state
    ///
    /// The loading status is set to `Loading`, the door state is set to `Loading`
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `LoadingStartedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_loading_started(&mut self, event: &LoadingStartedEvent) -> Result<(), DockManagerError> {
        self.loading_status.loading_status = LoadingStatus::Loading;
        self.door_state = DoorState::Loading;
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `LoadingCompletedEvent`, updating the loading status and door state
    ///
    /// The loading status is set to `Completed`, the door state is set to `LoadingCompleted`
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `LoadingCompletedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_loading_completed(&mut self, event: &LoadingCompletedEvent) -> Result<(), DockManagerError> {
        self.loading_status.loading_status = LoadingStatus::Completed;
        self.door_state = DoorState::LoadingCompleted;
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `SensorStateChangedEvent`, updating the corresponding sensor's value
    ///
    /// If the sensor is found in the door's sensor map, its value is updated
    /// The last updated timestamp is also set
    ///
    /// # Arguments
    ///
    /// * `event`: The `SensorStateChangedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully (even if the sensor was not found)
    fn handle_sensor_state_changed(&mut self, event: &SensorStateChangedEvent) -> Result<(), DockManagerError> {
        if let Some(sensor) = self.sensors.get_mut(&event.sensor_name) {
            sensor.update_value(event.new_value);
        }
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `DoorStateChangedEvent`, updating the door's state
    ///
    /// The door's state is updated to the new state specified in the event
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `DoorStateChangedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_door_state_changed(&mut self, event: &DoorStateChangedEvent) -> Result<(), DockManagerError> {
        self.door_state = event.clone().new_state;
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `LoadingStatusChangedEvent`, updating the door's loading status
    ///
    /// The door's loading status is updated to the new status specified in the event
    /// and the last updated timestamp is set
    ///
    /// # Arguments
    ///
    /// * `event`: The `LoadingStatusChangedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the event was handled successfully
    fn handle_loading_status_changed(&mut self, event: &LoadingStatusChangedEvent) -> Result<(), DockManagerError> {
        self.loading_status.loading_status = event.clone().new_status;
        self.last_updated = event.timestamp;
        Ok(())
    }

    /// Handles a `TrailerStateChangedEvent`, currently logging a debug message
    ///
    /// # Arguments
    ///
    /// * `event`: The `TrailerStateChangedEvent` to be handled
    ///
    /// # Returns
    ///
    /// * `Ok(())` 
    fn handle_trailer_state_changed(&mut self, event: &TrailerStateChangedEvent) -> Result<(), DockManagerError> {
        log::debug!("trailer state changed: {:?}", event);
        Ok(())
    }

    /// Sets the manual mode of the door
    ///
    /// Updates the `manual_mode` field and the `last_updated` timestamp
    ///
    /// # Arguments
    ///
    /// * `mode`: The new `ManualMode` to set for the door
    pub fn set_manual_mode(&mut self, mode: ManualMode) {
        self.manual_mode = mode;
        self.last_updated = chrono::Local::now().naive_local();
    }

    /// Sets the docking time to the current time
    pub fn set_docking_time(&mut self) {
        self.consolidated.docking_time = Some(chrono::Local::now().naive_local());
    }

    /// Clears the docking time
    pub fn clear_docking_time(&mut self) {
        self.consolidated.docking_time = None;
    }

    /// Calculates the duration since docking, if applicable
    pub fn docking_duration(&self) -> Option<chrono::Duration> {
        self.consolidated.docking_time.map(|docking_time| {
            chrono::Local::now().naive_local().signed_duration_since(docking_time)
        })
    }

    /// Sets the fault state of the door
    ///
    /// Updates the `fault_state` field and the `last_updated` timestamp
    ///
    /// # Arguments
    ///
    /// * `state`: The new `FaultState` to set for the door
    pub fn set_fault_state(&mut self, state: FaultState) {
        self.fault_state = state;
        self.last_updated = chrono::Local::now().naive_local();
    }


    // Updates the door's state based on information from the WMS
    ///
    /// This method processes a `WmsDoorStatus` object and updates the door's
    /// `assigned_shipment`, `loading_status`, and `wms_shipment_status` fields accordingly
    /// It also generates `DockDoorEvent`s for any changes in shipment assignment or loading status
    ///
    /// # Arguments
    ///
    /// * `wms_status`: The `WmsDoorStatus` object containing the updated information from the WMS
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<DockDoorEvent>)` A vector of events generated due to the WMS update
    /// * `Err(DockManagerError)` if there's an error parsing the loading status from the WMS data
    pub fn update_from_wms(&mut self, wms_status: &WmsDoorStatus) -> DockManagerResult<Vec<DockDoorEvent>> {
        let mut events = Vec::new();
        info!("WMS Door Status: {:?}", wms_status);
        if self.assigned_shipment.current_shipment != wms_status.assigned_shipment {
            let old_shipment = self.assigned_shipment.current_shipment.clone();
            self.assigned_shipment.current_shipment = wms_status.assigned_shipment.clone();

            if let Some(shipment_id) = &wms_status.assigned_shipment {
                events.push(DockDoorEvent::ShipmentAssigned(ShipmentAssignedEvent {
                    plant_id: wms_status.plant.clone(),
                    dock_name: self.dock_name.clone(),
                    shipment_id: shipment_id.clone(),
                    timestamp: chrono::Local::now().naive_local(),
                    previous_shipment: old_shipment,
                }));
            } else if let Some(previous_shipment) = old_shipment {
                events.push(DockDoorEvent::ShipmentUnassigned(ShipmentUnassignedEvent {
                    plant_id: wms_status.plant.clone(),
                    dock_name: self.dock_name.clone(),
                    shipment_id: previous_shipment,
                    timestamp: chrono::Local::now().naive_local(),
                }));
            }
        }

        let new_loading_status = LoadingStatus::from_str(&wms_status.loading_status)
            .unwrap_or(LoadingStatus::Idle);
        if self.loading_status.loading_status != new_loading_status {
            events.push(DockDoorEvent::LoadingStatusChanged(LoadingStatusChangedEvent {
                plant_id: wms_status.plant.clone(),
                dock_name: self.dock_name.clone(),
                old_status: self.loading_status.loading_status,
                new_status: new_loading_status,
                timestamp: chrono::Local::now().naive_local(),
            }));
            self.loading_status.loading_status = new_loading_status;
        }

        self.loading_status.wms_shipment_status = wms_status.wms_shipment_status.clone();
        if wms_status.is_preload.is_some() {
            self.consolidated.is_preload = wms_status.is_preload.unwrap();
        }

        Ok(events)
    }

    pub fn check_loading_readiness(&self) -> bool {
        self.door_position == DoorPosition::Open &&
            self.dock_lock_state == DockLockState::Engaged &&
            self.leveler_position == LevelerPosition::Extended &&
            self.fault_state == FaultState::NoFault &&
            self.trailer_state == TrailerState::Docked &&
            self.manual_mode == ManualMode::Disabled &&
            self.restraint_state == RestraintState::Locked &&
            self.trailer_position_state == TrailerPositionState::Proper &&
            !self.trailer_door_fault &&
            !self.dock_lock_fault &&
            !self.door_fault &&
            !self.emergency_stop &&
            !self.leveler_fault
    }
}
