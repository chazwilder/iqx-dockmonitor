//! # Database Services

//! This module provides the core functionality for interacting with databases within the IQX Dock Manager application. 
//! It includes the `DatabaseConnectionFactory` for managing database connections and the `DatabaseService` for performing 
//! various database operations such as inserting dock door events and fetching WMS data.

use std::collections::HashMap;
use std::sync::Arc;
use secrecy::ExposeSecret;
use tokio::sync::Mutex;
use crate::config::Settings;
use crate::errors::{DockManagerError, DockManagerResult};
use crate::models::{DbInsert, WmsDoorStatus, WmsEvent};
use crate::repositories::{DoorEventRepository, WmsStatusRepository, Repository};
use crate::services::DatabaseClient;

/// A factory for creating and managing database connections on a per-plant basis
pub struct DatabaseConnectionFactory {
    /// A thread-safe map storing database connections, keyed by plant ID
    connections: Arc<Mutex<HashMap<String, DatabaseClient>>>,
}

impl DatabaseConnectionFactory {
    /// Creates a new `DatabaseConnectionFactory`
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieves a database connection for the specified plant
    ///
    /// If a connection for the plant already exists in the map, it is returned
    /// Otherwise, a new connection is created based on the plant's configuration 
    /// and added to the map before being returned
    ///
    /// # Arguments
    ///
    /// * `plant_id`: The ID of the plant for which to retrieve the connection
    /// * `settings`: The application settings containing database configuration
    ///
    /// # Returns
    ///
    /// * `Ok(DatabaseClient)`: The database connection for the specified plant
    /// * `Err(DockManagerError)`: If the plant is not found in the settings or if there's an error creating the connection
    pub async fn get_connection(&self, plant_id: &str, settings: &Settings) -> DockManagerResult<DatabaseClient> {
        let mut connections = self.connections.lock().await;
        if let Some(client) = connections.get(plant_id) {
            Ok(client.clone())
        } else {
            let plant_settings = settings.get_plant(plant_id)
                .ok_or_else(|| DockManagerError::ConfigError(format!("Plant {} not found in settings", plant_id)))?;
            let client = DatabaseClient::new(
                &plant_settings.lgv_wms_database.connection_string().expose_secret(),
                &plant_settings.lgv_wms_database.app_name,
            ).await?;
            connections.insert(plant_id.to_string(), client.clone());
            Ok(client)
        }
    }
}

/// Provides services for interacting with both local and plant-specific WMS databases
pub struct DatabaseService {
    /// The database client for the local database
    local_client: DatabaseClient,
    /// A map of database clients for different plants, keyed by plant ID
    plant_clients: HashMap<String, DatabaseClient>,
    /// The application settings containing database configurations
    settings: Arc<Settings>,
}

impl DatabaseService {
    /// Creates a new `DatabaseService`
    ///
    /// Initializes the service by establishing connections to the local database and
    /// the WMS databases for each configured plant
    ///
    /// # Arguments
    ///
    /// * `settings`: The application settings containing database configurations
    ///
    /// # Returns
    ///
    /// * `Ok(Self)`: The initialized `DatabaseService` instance
    /// * `Err(DockManagerError)`: If there's an error establishing any of the database connections
    pub async fn new(settings: Arc<Settings>) -> DockManagerResult<Self> {
        let local_client = DatabaseClient::new(
            &settings.database.connection_string().expose_secret(),
            &settings.database.app_name,
        ).await?;

        let mut plant_clients = HashMap::new();
        for plant in &settings.plants {
            let client = DatabaseClient::new(
                &plant.lgv_wms_database.connection_string().expose_secret(),
                &plant.lgv_wms_database.app_name,
            ).await?;
            plant_clients.insert(plant.plant_id.clone(), client);
        }

        Ok(Self {
            local_client,
            plant_clients,
            settings,
        })
    }

    /// Inserts a batch of dock door events into the local database
    ///
    /// # Arguments
    ///
    /// * `events`: A vector of `DbInsert` objects representing the events to be inserted
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If the events were inserted successfully
    /// * `Err(DockManagerError)`: If there's an error during the insertion process
    pub async fn insert_dock_door_events(&self, events: Vec<DbInsert>) -> DockManagerResult<()> {
        let repo = DoorEventRepository::new(self.local_client.clone());
        for event in events {
            repo.insert(&event).await?;
        }
        Ok(())
    }

    /// Fetches WMS data (door statuses) for the specified plant
    ///
    /// # Arguments:
    /// * `plant_id`: The ID of the plant for which to fetch WMS data
    ///
    /// # Returns
    /// * `Ok(Vec<WmsDoorStatus>)`: If the WMS data was fetched successfully
    /// * `Err(DockManagerError)`: If the plant is not found or if there's an error fetching the data
    pub async fn fetch_wms_data(&self, plant_id: &str) -> DockManagerResult<Vec<WmsDoorStatus>> {
        let client = self.plant_clients.get(plant_id)
            .ok_or_else(|| DockManagerError::ConfigError(format!("Plant {} not found", plant_id)))?;
        let repo = WmsStatusRepository::new(client.clone());
        let query = self.settings.queries.wms_door_status.replace("{|}", plant_id);
        repo.fetch(&query).await
    }

    /// Fetches WMS events for the specified plant, shipment, and dock
    ///
    /// # Arguments
    ///
    /// * `plant_id`: The ID of the plant
    /// * `shipment_id`: The ID of the shipment
    /// * `dock_name`: The name of the dock
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<WmsEvent>)`: If the WMS events were fetched successfully
    /// * `Err(DockManagerError)` if the plant is not found or if there's an error fetching the events
    pub async fn fetch_wms_events(&self, plant_id: &str, shipment_id: &str, dock_name: &str) -> DockManagerResult<Vec<WmsEvent>> {
        let client = self.plant_clients.get(plant_id)
            .ok_or_else(|| DockManagerError::ConfigError(format!("Plant {} not found", plant_id)))?;
        let repo = WmsStatusRepository::new(client.clone());
        let query = self.settings.queries.wms_events
            .replace("{}", shipment_id)
            .replace("{|}", dock_name)
            .replace("{#}", plant_id);
        repo.fetch_wms_events(&query, shipment_id, dock_name).await
    }
}