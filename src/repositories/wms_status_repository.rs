use async_trait::async_trait;
use crate::models::{WmsDoorStatus, WmsEvent};
use crate::errors::DockManagerError;
use crate::services::DatabaseClient;
use crate::repositories::repository_trait::Repository;
use sqlx_oldapi::{Mssql};

/// A repository responsible for fetching WMS (Warehouse Management System) status and event data from the database
pub struct WmsStatusRepository {
    /// The database client used to interact with the database
    client: DatabaseClient,
}

impl WmsStatusRepository {
    /// Creates a new `WmsStatusRepository`
    ///
    /// # Arguments
    ///
    /// * `client`: The `DatabaseClient` to use for database operations
    pub fn new(client: DatabaseClient) -> Self {
        Self { client }
    }

    /// Fetches WMS events from the database based on the provided query
    ///
    /// The `_shipment_id` and `_dock_name` parameters are currently unused but might be intended for future filtering or logging
    ///
    /// # Arguments
    ///
    /// * `query`: The SQL query to execute for fetching the WMS events
    /// * `_shipment_id`: (Unused) The ID of the shipment (potentially for future filtering)
    /// * `_dock_name`: (Unused) The name of the dock (potentially for future filtering)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<WmsEvent>)`: A vector of `WmsEvent` representing the fetched WMS events
    /// * `Err(DockManagerError)`: If there's an error during the database operation
    pub async fn fetch_wms_events(&self, query: &str, _shipment_id: &str, _dock_name: &str) -> Result<Vec<WmsEvent>, DockManagerError> {
        sqlx_oldapi::query_as::<Mssql, WmsEvent>(query)
            .fetch_all(&*self.client.pool)
            .await
            .map_err(DockManagerError::DatabaseError)
    }
}

#[async_trait]
impl Repository<WmsDoorStatus> for WmsStatusRepository {
    /// Inserts a `WmsDoorStatus` into the database (currently not implemented)
    ///
    /// This method currently returns an error indicating that it's not implemented
    /// It might be intended for future use when inserting WMS door status data is required
    ///
    /// # Arguments
    ///
    /// * `_status`: The `WmsDoorStatus` to be inserted (currently unused)
    ///
    /// # Returns:
    /// * `Err(DockManagerError)` indicating that the operation is not implemented
    async fn insert(&self, _status: &WmsDoorStatus) -> Result<(), DockManagerError> {
        Err(DockManagerError::DatabaseError(sqlx_oldapi::Error::RowNotFound))
    }

    /// Fetches WMS door statuses from the database based on the provided query
    ///
    /// # Arguments
    ///
    /// * `query`: The SQL query to execute for fetching the WMS door statuses
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<WmsDoorStatus>)`: A vector of `WmsDoorStatus` representing the fetched door statuses
    /// * `Err(DockManagerError)`: If there's an error during the database operation
    async fn fetch(&self, query: &str) -> Result<Vec<WmsDoorStatus>, DockManagerError> {
        sqlx_oldapi::query_as::<Mssql, WmsDoorStatus>(query)
            .fetch_all(&*self.client.pool)
            .await
            .map_err(DockManagerError::DatabaseError)
    }
}