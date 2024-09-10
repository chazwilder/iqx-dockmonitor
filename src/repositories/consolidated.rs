use crate::errors::DockManagerError;
use crate::services::DatabaseClient;
use crate::repositories::repository_trait::Repository;
use async_trait::async_trait;
use sqlx_oldapi::Mssql;
use crate::models::consolidated_dock_event::ConsolidatedDockEvent;

/// A repository responsible for managing consolidated dock events in the database.
pub struct ConsolidatedDockEventRepository {
    /// The database client used to interact with the database.
    client: DatabaseClient,
}

impl ConsolidatedDockEventRepository {
    /// Creates a new `ConsolidatedDockEventRepository`.
    ///
    /// # Arguments
    /// * `client`: The `DatabaseClient` to use for database operations.
    pub fn new(client: DatabaseClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Repository<ConsolidatedDockEvent> for ConsolidatedDockEventRepository {
    /// Inserts a consolidated dock event into the database.
    ///
    /// # Arguments
    /// * `event`: The `ConsolidatedDockEvent` to be inserted
    ///
    /// # Returns
    /// * `Ok(())` if the insertion was successful
    /// * `Err(DockManagerError)` if there was an error during the database operation
    async fn insert(&self, event: &ConsolidatedDockEvent) -> Result<(), DockManagerError> {
        let query = r#"
            EXEC sp_GetDockDoorEventDetails @PLANT = @p1, @DOCK_DOOR = @p2, @SHIPMENT_ID = @p3, @PRELOAD = @p4;
        "#;

        sqlx_oldapi::query::<Mssql>(query)
            .bind(&event.plant)
            .bind(&event.door_name)
            .bind(&event.shipment_id)
            .bind(&event.is_preload)
            .execute(&*self.client.pool)
            .await
            .map_err(DockManagerError::DatabaseError)?;

        Ok(())
    }

    /// Fetches consolidated dock events from the database based on the provided query
    ///
    /// # Arguments
    /// * `query`: The SQL query to execute for fetching the events.
    ///
    /// # Returns
    /// * `Ok(Vec<ConsolidatedDockEvent>)`: A vector of `ConsolidatedDockEvent` representing the fetched consolidated dock events
    /// * `Err(DockManagerError)` if there was an error during the database operation
    async fn fetch(&self, query: &str) -> Result<Vec<ConsolidatedDockEvent>, DockManagerError> {
        sqlx_oldapi::query_as::<_, ConsolidatedDockEvent>(query)
            .fetch_all(&*self.client.pool)
            .await
            .map_err(DockManagerError::DatabaseError)
    }
}