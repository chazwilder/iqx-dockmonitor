//! # PLC Value Representation

//! This module defines the `PlcVal` struct, which represents a value read from a PLC (Programmable Logic Controller) sensor. 
//! It encapsulates the essential information associated with a sensor reading, including the plant ID, door name, door IP address, 
//! sensor name, the actual sensor value, and the timestamp of the reading.


use chrono::{NaiveDateTime, Local};
use serde::{Serialize, Deserialize};

/// Represents a value read from a PLC sensor.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlcVal {
    /// The ID of the plant where the sensor is located.
    pub plant_id: String,
    /// The name of the dock door associated with the sensor.
    pub door_name: String,
    /// The IP address of the dock door's PLC.
    pub door_ip: String,
    /// The name of the sensor.
    pub sensor_name: String,
    /// The value read from the sensor.
    pub value: u8,
    /// The timestamp when the sensor value was read.
    pub timestamp: NaiveDateTime,
}

impl PlcVal {
    /// Creates a new `PlcVal` instance.
    /// 
    /// # Arguments
    /// 
    /// * `plant_id`: The ID of the plant.
    /// * `door_name`: The name of the dock door.
    /// * `door_ip`: The IP address of the dock door's PLC.
    /// * `sensor_name`: The name of the sensor.
    /// * `value`: The sensor value.
    /// 
    /// # Returns
    /// 
    /// A new `PlcVal` instance with the provided information and the current local time as the timestamp.
    pub fn new(plant_id: &str, door_name: &str, door_ip: &str, sensor_name: &str, value: u8) -> Self {
        PlcVal {
            plant_id: plant_id.to_string(),
            door_name: door_name.to_string(),
            door_ip: door_ip.to_string(),
            sensor_name: sensor_name.to_string(),
            value,
            timestamp: Local::now().naive_local(),
        }
    }
}