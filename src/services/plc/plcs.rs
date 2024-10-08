use std::sync::Arc;
use std::time::{Duration, Instant};
use log::{info, error};
use futures::future::join_all;
use crate::models::PlcVal;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::config::Settings;
use tokio::time::timeout;
use crate::services::plc::plc_tag_factory::PlcTagFactory;
use crate::services::plc::plc_reader::PlcReader;

// noinspection all
/// # PlcService
///
/// The `PlcService` struct is responsible for managing communications with Programmable Logic Controllers (PLCs)
/// in the dock door management system. It handles sensor polling and data retrieval from the PLCs.
///
/// ## Fields
///
/// * `reader`: An `Arc<PlcReader>` that provides thread-safe access to the PLC reading functionality.
/// * `max_retries`: The maximum number of retry attempts for reading sensor data.
///
/// ## Usage
///
/// The `PlcService` is typically instantiated in the `main.rs` file or a central service manager.
/// It's used by the `DockDoorController` to periodically poll sensor data from the PLCs.
///
/// ## Example
///
/// ```rust
/// let plc_service = PlcService::new();
/// let sensor_data = plc_service.poll_sensors(&settings).await?;
/// ```
#[derive(Clone)]
pub struct PlcService {
    /// Provides thread-safe access to the PLC reading functionality
    reader: Arc<PlcReader>,
    /// The maximum number of retry attempts for reading sensor data
    max_retries: u32,
}

impl PlcService {
    // noinspection all
    /// Creates a new instance of `PlcService`.
    ///
    /// This method initializes a new `PlcService` with default configurations:
    /// - A `PlcReader` with a 5000ms timeout
    /// - A maximum of 3 retry attempts for sensor reading
    ///
    /// # Returns
    ///
    /// Returns a new `PlcService` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// let plc_service = PlcService::new();
    /// ```
    pub fn new() -> Self {
        Self {
            reader: Arc::new(PlcReader::new(5000)),
            max_retries: 3,
        }
    }

    /// Polls sensors across all plants and collects their values
    ///
    /// This method iterates through all configured plants, their associated doors, and sensors
    /// For each sensor, it attempts to read its value from the PLC using the `read_sensor` method
    /// The collected sensor values are returned as a vector of `PlcVal`
    ///
    /// # Arguments
    /// * `settings`: The application settings containing plant and sensor configurations
    ///
    /// # Returns
    /// * `Ok(Vec<PlcVal>)`: The collected sensor values
    /// * `Err(DockManagerError)`: If there's an error during sensor polling or task joining
    pub async fn poll_sensors(&self, settings: &Settings) -> DockManagerResult<Vec<PlcVal>> {
        let start = Instant::now();
        let mut all_plc_values = Vec::new();
        let _ = self.max_retries.clone();

        for plant in &settings.plants {
            let plant_start = Instant::now();
            let plant_id = plant.plant_id.clone();

            info!("Starting sensor polling for plant {} with {} doors", plant_id, plant.dock_doors.dock_door_config.len());

            let sensor_futures: Vec<_> = plant.dock_doors.dock_door_config.iter()
                .flat_map(|door| {
                    plant.dock_doors.dock_plc_tags.iter().map({
                        let plant = plant_id.clone();
                        move |sensor| {
                            let reader = Arc::clone(&self.reader);
                            let plant_id = plant.clone();
                            let door_name = door.dock_name.clone();
                            let door_ip = door.dock_ip.clone();
                            let sensor_name = sensor.tag_name.clone();
                            let address = sensor.address.clone();

                            tokio::spawn(async move {
                                timeout(Duration::from_secs(1), Self::read_sensor(
                                    reader,
                                    plant_id,
                                    door_name,
                                    door_ip,
                                    sensor_name,
                                    address,
                                )).await
                            })
                        }
                    })
                })
                .collect();

            let results = join_all(sensor_futures).await;

            for result in results {
                match result {
                    Ok(Ok(Ok(plc_val))) => all_plc_values.push(plc_val),
                    Ok(Ok(Err(e))) => info!("Sensor read error: {:?}", e),
                    Ok(Err(_)) => info!("Sensor read timed out"),
                    Err(e) => error!("Task join error: {:?}", e),
                }
            }

            info!("Completed sensor polling for plant {} in {:?}", plant_id, plant_start.elapsed());
        }

        info!("Completed polling all sensors in {:?}", start.elapsed());
        Ok(all_plc_values)
    }

    /// Attempts to read a sensor value from a PLC with retries
    ///
    /// This method creates a PLC tag using the `PlcTagFactory` and then tries to read its value using the `PlcReader`
    /// If the read fails, it retries up to `max_retries` times with a 2-second delay between attempts
    /// If all attempts fail it returns an error
    ///
    /// # Arguments
    ///
    /// * `reader`: The `PlcReader` used for communication with the PLC
    /// * `max_retries`: The maximum number of retry attempts
    /// * `plant_id`: The ID of the plant where the sensor is located
    /// * `door_name`: The name of the door associated with the sensor
    /// * `door_ip`: The IP address of the PLC controlling the door
    /// * `sensor`: The name of the sensor
    /// * `plc_tag_address`: The PLC address of the sensor
    ///
    /// # Returns
    ///
    /// * `Ok(PlcVal)`: The read sensor value encapsulated in a `PlcVal` struct
    /// * `Err(DockManagerError)`: If the sensor read fails after all retries
    async fn read_sensor(
        reader: Arc<PlcReader>,
        plant_id: String,
        door_name: String,
        door_ip: String,
        sensor: String,
        plc_tag_address: String
    ) -> DockManagerResult<PlcVal> {
        let tag = PlcTagFactory::create_tag(&door_ip, &plc_tag_address, 1000)
            .map_err(|e| DockManagerError::PlcError(format!(
                "Failed to create PLC tag for sensor '{}' (address: {}) on door '{}' in plant '{}': {:?}",
                sensor, plc_tag_address, door_name, plant_id, e
            )))?;

        reader.read_tag(tag).await
            .map(|value| PlcVal::new(&plant_id, &door_name, &door_ip, &sensor, value))
            .map_err(|e| DockManagerError::PlcError(format!(
                "Failed to read sensor '{}' (address: {}) on door '{}' in plant '{}': {:?}",
                sensor, plc_tag_address, door_name, plant_id, e
            )))
    }
}