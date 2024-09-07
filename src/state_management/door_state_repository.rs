use dashmap::DashMap;
use log::info;
use crate::config::Settings;
use crate::models::{DbInsert, DockDoor};
use crate::errors::DockManagerError;

/// Repository for managing the state of dock doors across multiple plants.
///
/// This struct uses a nested DashMap structure to efficiently store and retrieve
/// dock door information with concurrent access, organized by plant.
pub struct DoorStateRepository {
    plants: DashMap<String, DashMap<String, DockDoor>>,
}

impl DoorStateRepository {
    /// Creates a new, empty `DoorStateRepository`.
    ///
    /// # Returns
    ///
    /// A new instance of `DoorStateRepository`.
    pub fn new() -> Self {
        Self {
            plants: DashMap::new(),
        }
    }

    /// Retrieves a dock door state.
    ///
    /// # Arguments
    ///
    /// * `plant_id` - The ID of the plant.
    /// * `door_name` - The name of the door.
    ///
    /// # Returns
    ///
    /// An `Option<DockDoor>` which is `Some(DockDoor)` if the door exists, or `None` if it doesn't.
    pub fn get_door_state(&self, plant_id: &str, door_name: &str) -> Option<DockDoor> {
        self.plants.get(plant_id)
            .and_then(|plant_doors| plant_doors.get(door_name).map(|door| door.clone()))
    }

    /// Updates or inserts a dock door.
    ///
    /// # Arguments
    ///
    /// * `plant_id` - The ID of the plant.
    /// * `door` - The `DockDoor` instance to update or insert.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the operation was successful, or an `Err(DockManagerError)` if it failed.
    pub fn update_door(&self, plant_id: &str, door: DockDoor) -> Result<(), DockManagerError> {
        self.plants
            .entry(plant_id.to_string())
            .or_insert_with(DashMap::new)
            .insert(door.dock_name.clone(), door);
        Ok(())
    }

    /// Retrieves all dock doors for a specific plant.
    ///
    /// # Arguments
    ///
    /// * `plant_id` - The ID of the plant.
    ///
    /// # Returns
    ///
    /// A `Vec<DockDoor>` containing all dock doors for the specified plant.
    pub fn get_plant_doors(&self, plant_id: &str) -> Vec<DockDoor> {
        self.plants.get(plant_id)
            .map(|plant_doors| plant_doors.iter().map(|entry| entry.value().clone()).collect())
            .unwrap_or_default()
    }

    /// Retrieves all dock doors across all plants.
    ///
    /// # Returns
    ///
    /// A `Vec<DockDoor>` containing all dock doors in the repository.
    pub fn get_all_doors(&self) -> Vec<DockDoor> {
        self.plants.iter()
            .flat_map(|plant| {
                plant.value().iter().map(|door| door.value().clone()).collect::<Vec<_>>()
            })
            .collect()
    }

    /// Checks if a specific dock door exists in a plant.
    ///
    /// # Arguments
    ///
    /// * `plant_id` - The ID of the plant.
    /// * `door_name` - The name of the door.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the door exists (`true`) or not (`false`).
    pub fn door_exists(&self, plant_id: &str, door_name: &str) -> bool {
        self.plants.get(plant_id)
            .map(|plant_doors| plant_doors.contains_key(door_name))
            .unwrap_or(false)
    }

    /// Removes a dock door from the repository.
    ///
    /// # Arguments
    ///
    /// * `plant_id` - The ID of the plant.
    /// * `door_name` - The name of the door to remove.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok(())` if the door was successfully removed,
    /// or an `Err(DockManagerError)` if the door was not found.
    pub fn remove_door(&self, plant_id: &str, door_name: &str) -> Result<(), DockManagerError> {
        if let Some(plant_doors) = self.plants.get_mut(plant_id) {
            if plant_doors.remove(door_name).is_some() {
                return Ok(());
            }
        }
        Err(DockManagerError::DoorNotFound(format!("Plant: {}, Door: {}", plant_id, door_name)))
    }

    pub async fn insert_db_event(&self, plant_id: &str, event: DbInsert) -> Result<(), DockManagerError> {
        // Implement the logic to insert the DB event
        // This might involve updating the door state or storing the event separately
        info!("{:?} - {:?}", plant_id, event);
        Ok(())
    }

    /// Retrieves a list of all plant IDs in the repository.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing all plant IDs.
    pub fn get_plant_ids(&self) -> Vec<String> {
        self.plants.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Clears all doors from the repository.
    pub fn clear(&self) {
        self.plants.clear();
    }

    /// Returns the total number of doors across all plants in the repository.
    ///
    /// # Returns
    ///
    /// The number of doors as a `usize`.
    pub fn len(&self) -> usize {
        self.plants.iter().map(|plant| plant.len()).sum()
    }

    /// Checks if the repository is empty.
    ///
    /// # Returns
    ///
    /// `true` if the repository contains no doors in any plant, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.plants.is_empty() || self.plants.iter().all(|plant| plant.is_empty())
    }

    /// Initializes the repository with doors from the provided settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - The application settings containing door configurations.
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization is successful, or an `Err(DockManagerError)` if it fails.
    pub fn initialize_from_settings(&self, settings: &Settings) -> Result<(), DockManagerError> {
        for plant in &settings.plants {
            let plant_id = &plant.plant_id;
            let plant_doors = self.plants
                .entry(plant_id.clone())
                .or_insert_with(DashMap::new);

            for dock in &plant.dock_doors.dock_door_config {
                let door = DockDoor::new(
                    plant_id.clone(),
                    dock.dock_name.clone(),
                    dock.dock_ip.clone(),
                    plant,
                );
                plant_doors.insert(dock.dock_name.clone(), door);
            }
        }
        Ok(())
    }
}