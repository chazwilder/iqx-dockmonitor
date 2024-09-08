use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::models::{DbInsert, DockDoor};
use crate::errors::DockManagerError;
use crate::config::Settings;
use tracing::info;

pub struct DoorStateRepository {
    plants: Arc<RwLock<HashMap<String, HashMap<String, DockDoor>>>>,
}

impl DoorStateRepository {
    pub fn new() -> Self {
        Self {
            plants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_door_state(&self, plant_id: &str, door_name: &str) -> Option<DockDoor> {
        let plants = self.plants.read().await;
        plants.get(plant_id)
            .and_then(|plant_doors| plant_doors.get(door_name).cloned())
    }

    pub async fn update_door(&self, plant_id: &str, door: DockDoor) -> Result<(), DockManagerError> {
        let mut plants = self.plants.write().await;
        plants
            .entry(plant_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(door.dock_name.clone(), door);
        Ok(())
    }

    pub async fn get_all_doors(&self) -> Vec<DockDoor> {
        let plants = self.plants.read().await;
        plants.values()
            .flat_map(|plant_doors| plant_doors.values().cloned())
            .collect()
    }

    pub async fn initialize_from_settings(&self, settings: &Settings) -> Result<(), DockManagerError> {
        let mut plants = self.plants.write().await;
        for plant in &settings.plants {
            let plant_id = &plant.plant_id;
            let mut plant_doors = HashMap::new();

            for dock in &plant.dock_doors.dock_door_config {
                let door = DockDoor::new(
                    plant_id.clone(),
                    dock.dock_name.clone(),
                    dock.dock_ip.clone(),
                    plant,
                );
                plant_doors.insert(dock.dock_name.clone(), door);
            }

            plants.insert(plant_id.clone(), plant_doors);
        }
        Ok(())
    }

    pub async fn insert_db_event(&self, plant_id: &str, event: DbInsert) -> Result<(), DockManagerError> {
        info!("{:?} - {:?}", plant_id, event);
        Ok(())
    }
}