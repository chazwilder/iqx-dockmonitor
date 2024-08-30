
use crate::models::DbInsert;
use crate::errors::DockManagerError;
use crate::services::DatabaseClient;
use crate::repositories::repository_trait::Repository;
use async_trait::async_trait;
use sqlx_oldapi::{Mssql};

/// A repository responsible for managing dock door events in the database.
pub struct DoorEventRepository {
    /// The database client used to interact with the database.
    client: DatabaseClient,
}

impl DoorEventRepository {
    /// Creates a new `DoorEventRepository`.
    ///
    /// # Arguments
    /// * `client`: The `DatabaseClient` to use for database operations.
    pub fn new(client: DatabaseClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Repository<DbInsert> for DoorEventRepository {
    /// Inserts a dock door event into the database.
    ///
    /// # Arguments
    /// * `event`: The `DbInsert` representing the dock door event to be inserted
    /// 
    /// # Returns
    /// * `Ok(())` if the insertion was successful
    /// * `Err(DockManagerError)` if there was an error during the database operation
    async fn insert(&self, event: &DbInsert) -> Result<(), DockManagerError> {
        let query = r#"
            INSERT INTO DOCK_DOOR_EVENTS
            (LOG_DTTM, PLANT, DOOR_NAME, SHIPMENT_ID, EVENT_TYPE, SUCCESS, NOTES, ID_USER, SEVERITY, PREVIOUS_STATE, PREVIOUS_STATE_DTTM)
            VALUES
            (@p1, @p2, @p3, @p4, @p5, @p6, @p7, @p8, @p9, @p10, @p11)
        "#;

        sqlx_oldapi::query::<Mssql>(query)
            .bind(&event.LOG_DTTM)
            .bind(&event.PLANT)
            .bind(&event.DOOR_NAME)
            .bind(&event.SHIPMENT_ID)
            .bind(&event.EVENT_TYPE)
            .bind(&event.SUCCESS)
            .bind(&event.NOTES)
            .bind(&event.ID_USER)
            .bind(&event.SEVERITY)
            .bind(&event.PREVIOUS_STATE)
            .bind(&event.PREVIOUS_STATE_DTTM)
            .execute(&*self.client.pool)
            .await
            .map_err(DockManagerError::DatabaseError)?;

        Ok(())
    }

    /// Fetches dock door events from the database based on the provided query
    ///
    /// # Arguments
    /// * `query`: The SQL query to execute for fetching the events.
    /// 
    /// # Returns
    /// * `Ok(Vec<DbInsert>)`: A vector of `DbInsert` representing the fetched dock door events
    /// * `Err(DockManagerError)` if there was an error during the database operation
    async fn fetch(&self, query: &str) -> Result<Vec<DbInsert>, DockManagerError> {
        sqlx_oldapi::query_as::<_, DbInsert>(query)
            .fetch_all(&*self.client.pool)
            .await
            .map_err(DockManagerError::DatabaseError)
    }
}