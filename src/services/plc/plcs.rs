use std::sync::Arc;
use tokio::task;
use futures::future::try_join_all;
use crate::models::PlcVal;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::config::{Settings};
use crate::services::plc::plc_tag_factory::PlcTagFactory;
use crate::services::plc::plc_reader::PlcReader;



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
    //noinspection RsUnresolvedPath
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
        let plants = settings.plants.clone(); // Clone the plants data
        let mut all_plc_values = Vec::new();

        for plant in plants {
            let doors = plant.dock_doors.dock_door_config.clone();
            let sensors = plant.dock_doors.dock_plc_tags.clone();
            let reader = Arc::clone(&self.reader);
            let max_retries = self.max_retries;
            let plant_id = plant.plant_id.clone();

            let door_futures = doors.into_iter().map(|door| {
                let sensors = sensors.clone();
                let reader = Arc::clone(&reader);
                let plant_id = plant_id.clone();

                task::spawn(async move {
                    let sensor_futures = sensors.into_iter().map(|sensor| {
                        let door_clone = door.clone();
                        let reader = Arc::clone(&reader);
                        let plant_id = plant_id.clone();

                        task::spawn(async move {
                            Self::read_sensor(
                                &reader,
                                max_retries,
                                &plant_id,
                                &door_clone.dock_name,
                                &door_clone.dock_ip,
                                &sensor.tag_name,
                                &sensor.address
                            ).await
                        })
                    });

                    try_join_all(sensor_futures).await
                })
            });

            let results = try_join_all(door_futures).await
                .map_err(|e| DockManagerError::TaskJoinError(e.to_string()))?;

            let plant_plc_values: Result<Vec<_>, _> = results.into_iter()
                .flatten()
                .flatten()
                .collect();

            all_plc_values.extend(plant_plc_values?);
        }

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
        reader: &PlcReader,
        max_retries: u32,
        plant_id: &str,
        door_name: &str,
        door_ip: &str,
        sensor: &str,
        plc_tag_address: &str
    ) -> DockManagerResult<PlcVal> {
        for attempt in 0..max_retries {
            let tag = PlcTagFactory::create_tag(door_ip, plc_tag_address, reader.timeout_ms)?;
            match reader.read_tag(tag).await {
                Ok(value) => {
                    return     Ok(PlcVal::new(plant_id, door_name, door_ip, sensor, value));
                }
                Err(e) if attempt < max_retries - 1 => {
                    tracing::error!("Error reading sensor {} for door {}, attempt {}/{}: {:?}",
                           sensor, door_name, attempt + 1, max_retries, e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(DockManagerError::PlcError(format!("Failed to read sensor {} after {} attempts", sensor, max_retries)))
    }
}