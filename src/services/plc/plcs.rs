use std::sync::Arc;
use std::time::Instant;
use log::{info, error};
use futures::future::join_all;
use tokio::time::Duration;
use crate::models::PlcVal;
use crate::errors::DockManagerResult;
use crate::config::{Settings, DockPlcTag};
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
/// * `tag_factory`: An `Arc<PlcTagFactory>` for creating and caching PLC tags.
#[derive(Clone)]
pub struct PlcService {
    reader: Arc<PlcReader>,
    max_retries: u32,
    tag_factory: Arc<PlcTagFactory>,
}

impl PlcService {
    /// Creates a new instance of `PlcService`.
    ///
    /// This method initializes a new `PlcService` with default configurations:
    /// - A `PlcReader` with a 5000ms timeout
    /// - A maximum of 3 retry attempts for sensor reading
    /// - A new `PlcTagFactory` for tag creation and caching
    ///
    /// # Returns
    ///
    /// Returns a new `PlcService` instance.
    pub fn new() -> Self {
        Self {
            reader: Arc::new(PlcReader::new(5000)),
            max_retries: 3,
            tag_factory: Arc::new(PlcTagFactory::new()),
        }
    }

    /// Polls sensors across all plants and collects their values
    ///
    /// This method iterates through all configured plants and their associated doors.
    /// For each door, it reads all sensors in parallel using the `read_door_sensors` method.
    /// The collected sensor values are returned as a vector of `PlcVal`.
    ///
    /// # Arguments
    ///
    /// * `settings`: The application settings containing plant and sensor configurations
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PlcVal>)`: The collected sensor values
    /// * `Err(DockManagerError)`: If there's an error during sensor polling or task joining
    pub async fn poll_sensors(&self, settings: &Settings) -> DockManagerResult<Vec<PlcVal>> {
        let start = Instant::now();
        let mut all_plc_values = Vec::new();

        for (plant_index, plant) in settings.plants.iter().enumerate() {
            let plant_start = Instant::now();
            let plant_id = plant.plant_id.clone();

            info!("Starting sensor polling for plant {} with {} doors", plant_index, plant.dock_doors.dock_door_config.len());

            let door_futures: Vec<_> = plant.dock_doors.dock_door_config.iter().map(|door| {
                let reader = Arc::clone(&self.reader);
                let max_retries = self.max_retries;
                let plant_id = plant_id.clone();
                let door_name = door.dock_name.clone();
                let door_ip = door.dock_ip.clone();
                let dock_plc_tags = plant.dock_doors.dock_plc_tags.clone();
                let tag_factory = Arc::clone(&self.tag_factory);

                tokio::spawn(async move {
                    Self::read_door_sensors(
                        reader,
                        tag_factory,
                        max_retries,
                        plant_id,
                        door_name,
                        door_ip,
                        dock_plc_tags,
                    ).await
                })
            }).collect();

            let results = join_all(door_futures).await;
            all_plc_values.extend(results.into_iter().filter_map(|r| r.ok()).flatten());

            info!("Completed sensor polling for plant {} in {:?}", plant_index, plant_start.elapsed());
        }

        info!("Completed polling all sensors in {:?}", start.elapsed());
        Ok(all_plc_values.into_iter().flatten().collect())
    }

    /// Reads all sensors for a single door
    ///
    /// This method attempts to read the values of all sensors associated with a door.
    /// It uses the `PlcTagFactory` to create or retrieve cached tags for each sensor.
    ///
    /// # Arguments
    ///
    /// * `reader`: The `PlcReader` used for communication with the PLC
    /// * `tag_factory`: The `PlcTagFactory` used for creating and caching PLC tags
    /// * `max_retries`: The maximum number of retry attempts
    /// * `plant_id`: The ID of the plant where the door is located
    /// * `door_name`: The name of the door
    /// * `door_ip`: The IP address of the PLC controlling the door
    /// * `dock_plc_tags`: The configurations for all sensors on this door
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PlcVal>)`: The read sensor values encapsulated in `PlcVal` structs
    /// * `Err(DockManagerError)`: If the sensor reads fail after all retries
    async fn read_door_sensors(
        reader: Arc<PlcReader>,
        tag_factory: Arc<PlcTagFactory>,
        max_retries: u32,
        plant_id: String,
        door_name: String,
        door_ip: String,
        dock_plc_tags: Vec<DockPlcTag>,
    ) -> DockManagerResult<Vec<PlcVal>> {
        let mut plc_values = Vec::new();

        for sensor in dock_plc_tags {
            for attempt in 0..max_retries {
                let tag = tag_factory.get_or_create_tag(&door_ip, &sensor.address, reader.timeout_ms).await?;
                match reader.read_tag(tag).await {
                    Ok(value) => {
                        plc_values.push(PlcVal::new(&plant_id, &door_name, &door_ip, &sensor.tag_name, value));
                        break;
                    }
                    Err(e) => {
                        error!("Error reading sensor {} for door {} on attempt {}/{}: {:?}",
                           sensor.tag_name, door_name, attempt + 1, max_retries, e);
                        if attempt < max_retries - 1 {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        }

        Ok(plc_values)
    }
}