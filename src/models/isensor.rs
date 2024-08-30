//! # Dock Sensor Representation

//! This module defines the `DockSensor` enum and the `SensorData` struct, which together represent the various sensors 
//! associated with a dock door and the data collected from those sensors. The `SensorType` enum provides a type-safe 
//! way to identify different sensor types.

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// Represents the different types of sensors that can be associated with a dock door.
/// Each sensor type holds its specific `SensorData`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DockSensor {
    /// Sensor indicating if the dock is automatically disengaging.
    AutoDisengaging(SensorData),
    /// Sensor indicating if the dock is automatically engaging.
    AutoEngaging(SensorData),
    /// Sensor indicating the presence of a fault.
    FaultPresence(SensorData),
    /// Sensor indicating a fault with the trailer doors.
    FaultTrailerDoors(SensorData),
    /// Sensor indicating if the dock leveler is stored.
    RhDockLvlrStored(SensorData),
    /// Sensor indicating if the dock is ready.
    RhDockReady(SensorData),
    /// Sensor indicating a fault with the dock lock.
    RhDokLockFault(SensorData),
    /// Sensor indicating if the door is closed.
    RhDoorClosed(SensorData),
    /// Sensor indicating a fault with the door.
    RhDoorFault(SensorData),
    /// Sensor indicating if the door is open.
    RhDoorOpen(SensorData),
    /// Sensor indicating an emergency stop.
    RhEstop(SensorData),
    /// Sensor indicating a fault with the leveler.
    RhLevelerFault(SensorData),
    /// Sensor indicating if the leveler is ready.
    RhLevelrReady(SensorData),
    /// Sensor indicating if the dock is in manual mode.
    RhManualMode(SensorData),
    /// Sensor indicating if the restraint is engaged.
    RhRestraintEngaged(SensorData),
    /// Sensor indicating if the restraint is unlocked.
    RhRestraintUnlkd(SensorData),
    /// Inside vertical sensor, left side (A).
    SensorInsideVerticalLeftA(SensorData),
    /// Inside vertical sensor, right side (B).
    SensorInsideVerticalRightB(SensorData),
    /// Outside lateral sensor, left side (C).
    SensorOutsideLateralLeftC(SensorData),
    /// Outside lateral sensor, right side (D).
    SensorOutsideLateralRightD(SensorData),
    /// Outside vertical sensor, left side (E).
    SensorOutsideVerticalLeftE(SensorData),
    /// Outside vertical sensor, right side (F).
    SensorOutsideVerticalRightF(SensorData),
    /// Sensor measuring the trailer angle.
    TrailerAngle(SensorData),
    /// Sensor indicating if a trailer is at the door
    TrailerAtDoor(SensorData),
    /// Sensor indicating if the trailer is centered.
    TrailerCentering(SensorData),
    /// Sensor measuring the distance to the trailer
    TrailerDistance(SensorData),
}

/// Holds the data associated with a specific sensor reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    /// The name of the dock door the sensor belongs to
    pub door_name: String,
    /// The IP address of the PLC controlling the door
    pub door_ip: String,
    /// The name or type of the sensor
    pub sensor: String,
    /// The PLC address where the sensor's value is stored
    pub address: String,
    /// The current value read from the sensor
    pub current_value: Option<u8>,
    /// The previous value read from the sensor
    pub previous_value: Option<u8>,
    /// The timestamp of the last sensor update
    pub last_updated: NaiveDateTime,
    /// Information about the last reported change in the sensor's value
    pub last_reported_change: Option<(Option<u8>, Option<u8>)>,
}

/// Provides a type-safe representation of the different sensor types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SensorType {
    AutoDisengaging,
    AutoEngaging,
    FaultPresence,
    FaultTrailerDoors,
    RhDockLvlrStored,
    RhDockReady,
    RhDokLockFault,
    RhDoorClosed,
    RhDoorFault,
    RhDoorOpen,
    RhEstop,
    RhLevelerFault,
    RhLevelrReady,
    RhManualMode,
    RhRestraintEngaged,
    RhRestraintUnlkd,
    SensorInsideVerticalLeftA,
    SensorInsideVerticalRightB,
    SensorOutsideLateralLeftC,
    SensorOutsideLateralRightD,
    SensorOutsideVerticalLeftE,
    SensorOutsideVerticalRightF,
    TrailerAngle,
    TrailerAtDoor,
    TrailerCentering,
    TrailerDistance,
}

impl std::str::FromStr for SensorType {
    type Err = String;

    /// Converts a string representation into a `SensorType`.
    ///
    /// This method attempts to parse the input string (case-insensitively) and match it to a corresponding `SensorType` variant.
    /// If a match is found, it returns the `SensorType`; otherwise, it returns an error indicating an unknown sensor type.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str()  {
            "AUTO_DISENGAGING" => Ok(SensorType::AutoDisengaging),
            "AUTO_ENGAGING" => Ok(SensorType::AutoEngaging),
            "FAULT_PRESENCE" => Ok(SensorType::FaultPresence),
            "FAULT_TRAILER_DOORS" => Ok(SensorType::FaultTrailerDoors),
            "RH_DOCK_LVLR_STORED" => Ok(SensorType::RhDockLvlrStored),
            "RH_DOCK_READY" => Ok(SensorType::RhDockReady),
            "RH_DOKLOCK_FAULT" => Ok(SensorType::RhDokLockFault),
            "RH_DOOR_CLOSED" => Ok(SensorType::RhDoorClosed),
            "RH_DOOR_FAULT" => Ok(SensorType::RhDoorFault),
            "RH_DOOR_OPEN" => Ok(SensorType::RhDoorOpen),
            "RH_ESTOP" => Ok(SensorType::RhEstop),
            "RH_LEVELER_FAULT" => Ok(SensorType::RhLevelerFault),
            "RH_LEVELR_READY" => Ok(SensorType::RhLevelrReady),
            "RH_MANUAL_MODE" => Ok(SensorType::RhManualMode),
            "RH_RESTRAINT_ENGAGED" => Ok(SensorType::RhRestraintEngaged),
            "RH_RESTRAINT_UNLKD" => Ok(SensorType::RhRestraintUnlkd),
            "SENSOR_INSIDE_VERTICAL_LEFT_A" => Ok(SensorType::SensorInsideVerticalLeftA),
            "SENSOR_INSIDE_VERTICAL_RIGHT_B" => Ok(SensorType::SensorInsideVerticalRightB),
            "SENSOR_OUTSIDE_LATERAL_LEFT_C" => Ok(SensorType::SensorOutsideLateralLeftC),
            "SENSOR_OUTSIDE_LATERAL_RIGHT_D" => Ok(SensorType::SensorOutsideLateralRightD),
            "SENSOR_OUTSIDE_VERTICAL_LEFT_E" => Ok(SensorType::SensorOutsideVerticalLeftE),
            "SENSOR_OUTSIDE_VERTICAL_RIGHT_F" => Ok(SensorType::SensorOutsideVerticalRightF),
            "TRAILER_ANGLE" => Ok(SensorType::TrailerAngle),
            "TRAILER_AT_DOOR" => Ok(SensorType::TrailerAtDoor),
            "TRAILER_CENTERING" => Ok(SensorType::TrailerCentering),
            "TRAILER_DISTANCE" => Ok(SensorType::TrailerDistance),
            _ => Err(format!("Unknown sensor type: {}", s)),
        }
    }
}

impl DockSensor {
    /// Creates a new `DockSensor` instance based on the provided sensor type and data
    ///
    /// # Arguments
    ///
    /// * `door_name`: The name of the dock door the sensor belongs to
    /// * `door_ip`: The IP address of the PLC controlling the door
    /// * `sensor_type`: A string representing the type of sensor
    /// * `address`: The PLC address where the sensor's value is stored
    ///
    /// # Returns
    ///
    /// A new `DockSensor` instance of the appropriate variant, containing the provided `SensorData`
    ///
    /// # Panics
    ///
    /// This function will panic if an unknown `sensor_type` is provided
    pub fn new(door_name: &str, door_ip: &str, sensor_type: &str, address: &str) -> Self {
        let sensor_data = SensorData {
            door_name: door_name.to_string(),
            door_ip: door_ip.to_string(),
            sensor: sensor_type.to_string(),
            address: address.to_string(),
            current_value: None,
            previous_value: None,
            last_updated: Local::now().naive_local(),
            last_reported_change: None,
        };
        match sensor_type {
            "AUTO_DISENGAGING" => DockSensor::AutoDisengaging(sensor_data),
            "AUTO_ENGAGING" => DockSensor::AutoEngaging(sensor_data),
            "FAULT_PRESENCE" => DockSensor::FaultPresence(sensor_data),
            "FAULT_TRAILER_DOORS" => DockSensor::FaultTrailerDoors(sensor_data),
            "RH_DOCK_LVLR_STORED" => DockSensor::RhDockLvlrStored(sensor_data),
            "RH_DOCK_READY" => DockSensor::RhDockReady(sensor_data),
            "RH_DOkLOCK_FAULT" => DockSensor::RhDokLockFault(sensor_data),
            "RH_DOOR_CLOSED" => DockSensor::RhDoorClosed(sensor_data),
            "RH_DOOR_FAULT" => DockSensor::RhDoorFault(sensor_data),
            "RH_DOOR_OPEN" => DockSensor::RhDoorOpen(sensor_data),
            "RH_ESTOP" => DockSensor::RhEstop(sensor_data),
            "RH_LEVELER_FAULT" => DockSensor::RhLevelerFault(sensor_data),
            "RH_LEVELR_READY" => DockSensor::RhLevelrReady(sensor_data),
            "RH_MANUAL_MODE" => DockSensor::RhManualMode(sensor_data),
            "RH_RESTRAINT_ENGAGED" => DockSensor::RhRestraintEngaged(sensor_data),
            "RH_RESTRAINT_UNLKD" => DockSensor::RhRestraintUnlkd(sensor_data),
            "SENSOR_INSIDE_VERTICAL_LEFT_A" => DockSensor::SensorInsideVerticalLeftA(sensor_data),
            "SENSOR_INSIDE_VERTICAL_RIGHT_B" => DockSensor::SensorInsideVerticalRightB(sensor_data),
            "SENSOR_OUTSIDE_LATERAL_LEFT_C" => DockSensor::SensorOutsideLateralLeftC(sensor_data),
            "SENSOR_OUTSIDE_LATERAL_RIGHT_D" => DockSensor::SensorOutsideLateralRightD(sensor_data),
            "SENSOR_OUTSIDE_VERTICAL_LEFT_E" => DockSensor::SensorOutsideVerticalLeftE(sensor_data),
            "SENSOR_OUTSIDE_VERTICAL_RIGHT_F" => DockSensor::SensorOutsideVerticalRightF(sensor_data),
            "TRAILER_ANGLE" => DockSensor::TrailerAngle(sensor_data),
            "TRAILER_AT_DOOR" => DockSensor::TrailerAtDoor(sensor_data),
            "TRAILER_CENTERING" => DockSensor::TrailerCentering(sensor_data),
            "TRAILER_DISTANCE" => DockSensor::TrailerDistance(sensor_data),
            _ => panic!("Unknown sensor type: {}", sensor_type)
        }
    }

    /// Updates the sensor's value and metadata
    ///
    /// # Arguments
    ///
    /// * `new_value`: The new value read from the sensor
    pub fn update_value(&mut self, new_value: Option<u8>) {
        let sensor_data = self.get_sensor_data_mut();
        sensor_data.previous_value = sensor_data.current_value;
        sensor_data.current_value = new_value;
        sensor_data.last_updated = Local::now().naive_local();
    }

    /// Provides immutable access to the sensor's data
    ///
    /// # Returns
    ///
    /// A reference to the `SensorData` associated with this `DockSensor`
    pub fn get_sensor_data(&self) -> &SensorData {
        match self {
            DockSensor::AutoDisengaging(data) => data,
            DockSensor::AutoEngaging(data) => data,
            DockSensor::FaultPresence(data) => data,
            DockSensor::FaultTrailerDoors(data) => data,
            DockSensor::RhDockLvlrStored(data) => data,
            DockSensor::RhDockReady(data) => data,
            DockSensor::RhDokLockFault(data) => data,
            DockSensor::RhDoorClosed(data) => data,
            DockSensor::RhDoorFault(data) => data,
            DockSensor::RhDoorOpen(data) => data,
            DockSensor::RhEstop(data) => data,
            DockSensor::RhLevelerFault(data) => data,
            DockSensor::RhLevelrReady(data) => data,
            DockSensor::RhManualMode(data) => data,
            DockSensor::RhRestraintEngaged(data) => data,
            DockSensor::RhRestraintUnlkd(data) => data,
            DockSensor::SensorInsideVerticalLeftA(data) => data,
            DockSensor::SensorInsideVerticalRightB(data) => data,
            DockSensor::SensorOutsideLateralLeftC(data) => data,
            DockSensor::SensorOutsideLateralRightD(data) => data,
            DockSensor::SensorOutsideVerticalLeftE(data) => data,
            DockSensor::SensorOutsideVerticalRightF(data) => data,
            DockSensor::TrailerAngle(data) => data,
            DockSensor::TrailerAtDoor(data) => data,
            DockSensor::TrailerCentering(data) => data,
            DockSensor::TrailerDistance(data) => data,
        }
    }

    /// Provides mutable access to the sensor's data (for internal use)
    pub(crate) fn get_sensor_data_mut(&mut self) -> &mut SensorData {
        match self {
            DockSensor::AutoDisengaging(data) => data,
            DockSensor::AutoEngaging(data) => data,
            DockSensor::FaultPresence(data) => data,
            DockSensor::FaultTrailerDoors(data) => data,
            DockSensor::RhDockLvlrStored(data) => data,
            DockSensor::RhDockReady(data) => data,
            DockSensor::RhDokLockFault(data) => data,
            DockSensor::RhDoorClosed(data) => data,
            DockSensor::RhDoorFault(data) => data,
            DockSensor::RhDoorOpen(data) => data,
            DockSensor::RhEstop(data) => data,
            DockSensor::RhLevelerFault(data) => data,
            DockSensor::RhLevelrReady(data) => data,
            DockSensor::RhManualMode(data) => data,
            DockSensor::RhRestraintEngaged(data) => data,
            DockSensor::RhRestraintUnlkd(data) => data,
            DockSensor::SensorInsideVerticalLeftA(data) => data,
            DockSensor::SensorInsideVerticalRightB(data) => data,
            DockSensor::SensorOutsideLateralLeftC(data) => data,
            DockSensor::SensorOutsideLateralRightD(data) => data,
            DockSensor::SensorOutsideVerticalLeftE(data) => data,
            DockSensor::SensorOutsideVerticalRightF(data) => data,
            DockSensor::TrailerAngle(data) => data,
            DockSensor::TrailerAtDoor(data) => data,
            DockSensor::TrailerCentering(data) => data,
            DockSensor::TrailerDistance(data) => data,
        }
    }
}

