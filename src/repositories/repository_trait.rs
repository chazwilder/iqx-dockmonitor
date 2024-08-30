use async_trait::async_trait;
use sqlx_oldapi::FromRow;
use crate::errors::DockManagerError;

/// Defines a generic asynchronous repository interface for interacting with the database
#[async_trait]
pub trait Repository<T>
    where
        T: for<'r> FromRow<'r, sqlx_oldapi::mssql::MssqlRow> + Send + Unpin,
{
    /// Inserts a single item into the database
    ///
    /// # Arguments
    ///
    /// * `item`: A reference to the item of type `T` to be inserted
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the insertion is successful
    /// * `Err(DockManagerError)` if an error occurs during the insertion
    async fn insert(&self, item: &T) -> Result<(), DockManagerError>;

    /// Fetches data from the database based on the provided query
    ///
    /// The query should be a valid SQL query that returns rows of data that can be deserialized into the type `T`
    ///
    /// # Arguments
    ///
    /// * `query`: The SQL query string
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<T>)`: A vector containing the fetched rows, deserialized into the type `T`
    /// * `Err(DockManagerError)` if an error occurs during the fetch operation or deserialization
    async fn fetch(&self, query: &str) -> Result<Vec<T>, DockManagerError>;
}