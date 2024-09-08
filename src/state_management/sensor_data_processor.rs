use std::sync::Arc;
use log::{info, debug};
use crate::errors::{DockManagerError, DockManagerResult};
use crate::models::{
    DockDoor, DockDoorEvent, DockLockState, DoorPosition, DoorState, DoorStateChangedEvent,
    FaultState, LevelerPosition, ManualMode, PlcVal, RestraintState, SensorStateChangedEvent,
    TrailerPositionState, TrailerState, TrailerStateChangedEvent
};
use crate::state_management::door_state_repository::DoorStateRepository;

/// Processes sensor data updates for the dock monitoring system.
pub struct SensorDataProcessor {
    /// Repository for managing dock door states.
    door_repository: Arc<DoorStateRepository>,
}

impl SensorDataProcessor {
    /// Creates a new `SensorDataProcessor`.
    ///
    /// # Arguments
    ///
    /// * `door_repository` - A reference to the `DoorStateRepository` for managing door states.
    ///
    /// # Returns
    ///
    /// A new instance of `SensorDataProcessor`.
    pub fn new(door_repository: Arc<DoorStateRepository>) -> Self {
        Self {
            door_repository,
        }
    }

    /// Processes a batch of sensor updates and generates corresponding events.
    ///
    /// # Arguments
    ///
    /// * `sensor_values` - A vector of `PlcVal` representing the sensor updates.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of `DockDoorEvent`s generated from the sensor updates,
    /// or a `DockManagerError` if processing fails.
    pub async fn process_sensor_updates(&self, sensor_values: Vec<PlcVal>) -> DockManagerResult<Vec<DockDoorEvent>> {
        let mut events = Vec::new();
        for sensor_value in sensor_values {
            let plant_id = &sensor_value.plant_id;
            let door_name = &sensor_value.door_name;

            let mut door = self.door_repository.get_door_state(plant_id, door_name).await
                .ok_or_else(|| DockManagerError::DoorNotFound(format!("Plant: {}, Door: {}", plant_id, door_name)))?;

            let new_events = self.process_single_sensor_update(&mut door, &sensor_value).await?;
            events.extend(new_events);

            self.door_repository.update_door(plant_id, door).await?;
        }

        Ok(events)
    }

    /// Processes a single sensor update for a specific door.
    ///
    /// # Arguments
    ///
    /// * `door` - A mutable reference to the `DockDoor` being updated.
    /// * `sensor_value` - The `PlcVal` containing the sensor update.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of `DockDoorEvent`s generated from the sensor update,
    /// or a `DockManagerError` if processing fails.
    async fn process_single_sensor_update(&self, door: &mut DockDoor, sensor_value: &PlcVal) -> Result<Vec<DockDoorEvent>, DockManagerError> {
        let mut events = Vec::new();

        let sensor_evaluation = door.update_sensor(&sensor_value.sensor_name, Some(sensor_value.value))?;

        if sensor_evaluation.changed {
            if sensor_evaluation.old_value.is_none() {
                debug!("Skipping initial sensor update for door: {}, sensor: {}", door.dock_name, sensor_value.sensor_name);
                self.update_door_state(door, sensor_value, &mut Vec::new())?;
                return Ok(Vec::new())
            } else {
                events.push(DockDoorEvent::SensorStateChanged(SensorStateChangedEvent {
                    plant_id: door.plant_id.clone(),
                    dock_name: door.dock_name.clone(),
                    sensor_name: sensor_value.sensor_name.clone(),
                    old_value: sensor_evaluation.old_value,
                    new_value: sensor_evaluation.new_value,
                    timestamp: chrono::Local::now().naive_local(),
                }));
                self.update_door_state(door, sensor_value, &mut events)?;
            }
        }

        Ok(events)
    }

    /// Updates the door state based on the sensor value.
    ///
    /// # Arguments
    ///
    /// * `door` - A mutable reference to the `DockDoor` being updated.
    /// * `sensor_value` - The `PlcVal` containing the sensor update.
    /// * `events` - A mutable reference to the vector of events being generated.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a `DockManagerError` if processing fails.
    fn update_door_state(&self, door: &mut DockDoor, sensor_value: &PlcVal, events: &mut Vec<DockDoorEvent>) -> Result<(), DockManagerError> {
        match sensor_value.sensor_name.as_str() {
            "AUTO_DISENGAGING" => {
                door.restraint_state = if sensor_value.value == 1 { RestraintState::Unlocking } else { RestraintState::Unlocked };
            },
            "AUTO_ENGAGING" => {
                door.restraint_state = if sensor_value.value == 1 { RestraintState::Locking } else { RestraintState::Locked };
            },
            "FAULT_PRESENCE" => {
                door.fault_state = if sensor_value.value == 1 { FaultState::FaultPresent } else { FaultState::NoFault };
            },
            "FAULT_TRAILER_DOORS" => {
                door.trailer_door_fault = sensor_value.value == 1;
            },
            "RH_DOCK_READY" => {
                if sensor_value.value == 1 &&
                    (door.door_state == DoorState::TrailerDocked || door.door_state == DoorState::Unassigned) {
                    if let Some(old_value) = door.sensors.get("RH_DOCK_READY").and_then(|s| s.get_sensor_data().current_value) {
                        if old_value == 0 {
                            self.change_door_state(door, DoorState::DoorReady, events)?;
                        }
                    }
                }
            },
            "RH_DOKLOCK_FAULT" => {
                door.dock_lock_fault = sensor_value.value == 1;
            },
            "RH_DOOR_FAULT" => {
                door.door_fault = sensor_value.value == 1;
            },
            "RH_DOOR_OPEN" => {
                door.door_position = if sensor_value.value == 1 { DoorPosition::Open } else { DoorPosition::Closed };
            },
            "RH_ESTOP" => {
                door.emergency_stop = sensor_value.value == 1;
                if door.emergency_stop {
                    door.manual_mode = ManualMode::Enabled;
                }
            },
            "RH_LEVELER_FAULT" => {
                door.leveler_fault = sensor_value.value == 1;
            },
            "RH_LEVELR_READY" => {
                door.leveler_position = if sensor_value.value == 1 { LevelerPosition::Extended } else { LevelerPosition::Stored };
            },
            "RH_MANUAL_MODE" => {
                door.manual_mode = if sensor_value.value == 1 { ManualMode::Enabled } else { ManualMode::Disabled };
            },
            "RH_RESTRAINT_ENGAGED" => {
                door.dock_lock_state = if sensor_value.value == 1 { DockLockState::Engaged } else { DockLockState::Disengaged };
            },
            "TRAILER_ANGLE" | "TRAILER_CENTERING" | "TRAILER_DISTANCE" => {
                door.trailer_position_state = if sensor_value.value == 0 { TrailerPositionState::Proper } else { TrailerPositionState::Improper };
            },
            "TRAILER_AT_DOOR" => {
                let new_trailer_state = if sensor_value.value == 1 {
                    door.set_docking_time();
                    TrailerState::Docked
                } else {
                    door.clear_docking_time();
                    TrailerState::Undocked
                };
                if door.trailer_state != new_trailer_state {
                    info!("Trailer state change detected: {:?} -> {:?}", door.trailer_state, new_trailer_state);
                    events.push(DockDoorEvent::TrailerStateChanged(TrailerStateChangedEvent {
                        plant_id: door.plant_id.clone(),
                        dock_name: door.dock_name.clone(),
                        old_state: door.trailer_state,
                        new_state: new_trailer_state,
                        timestamp: chrono::Local::now().naive_local(),
                    }));
                    door.trailer_state = new_trailer_state;
                    if new_trailer_state == TrailerState::Docked {
                        info!("Changing door state to TrailerDocked");
                        self.change_door_state(door, DoorState::TrailerDocked, events)?;
                    }
                }
            },
            _ => {
                debug!("Unhandled sensor type: {}", sensor_value.sensor_name);
            }
        }
        Ok(())
    }


    /// Changes the door state and generates a DoorStateChanged event.
    ///
    /// # Arguments
    ///
    /// * `door` - A mutable reference to the `DockDoor` being updated.
    /// * `new_state` - The new `DoorState` to set.
    /// * `events` - A mutable reference to the vector of events being generated.
    ///
    /// # Returns
    ///
    /// A Result indicating success or a `DockManagerError` if processing fails.
    fn change_door_state(&self, door: &mut DockDoor, new_state: DoorState, events: &mut Vec<DockDoorEvent>) -> Result<(), DockManagerError> {
        if door.door_state != new_state {
            let old_state = door.door_state;
            door.door_state = new_state;
            events.push(DockDoorEvent::DoorStateChanged(DoorStateChangedEvent {
                plant_id: door.plant_id.clone(),
                dock_name: door.dock_name.clone(),
                old_state,
                new_state,
                timestamp: chrono::Local::now().naive_local(),
            }));
            info!("Door state changed for {}: {:?} -> {:?}", door.dock_name, old_state, new_state);
        }
        Ok(())
    }
}