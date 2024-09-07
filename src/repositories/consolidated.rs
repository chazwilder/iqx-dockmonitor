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
            INSERT INTO TPT.IQX_CONSOLIDATED_DOCK_EVENTS
            (PLANT, DOOR_NAME, SHIPMENT_ID, DOCKING_TIME_MINUTES, INSPECTION_TIME_MINUTES,
             ENQUEUED_TIME_MINUTES, SHIPMENT_ASSIGNED, DOCK_ASSIGNMENT, TRAILER_DOCKING,
             STARTED_SHIPMENT, LGV_START_LOADING, DOCK_READY, IS_PRELOAD)
            VALUES
            (@p1, @p2, @p3, @p4, @p5, @p6, @p7, @p8, @p9, @p10, @p11, @p12, @p13)
        "#;

        sqlx_oldapi::query::<Mssql>(query)
            .bind(&event.plant)
            .bind(&event.door_name)
            .bind(&event.shipment_id)
            .bind(&event.docking_time_minutes)
            .bind(&event.inspection_time_minutes)
            .bind(&event.enqueued_time_minutes)
            .bind(&event.shipment_assigned)
            .bind(&event.dock_assignment)
            .bind(&event.trailer_docking)
            .bind(&event.started_shipment)
            .bind(&event.lgv_start_loading)
            .bind(&event.dock_ready)
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