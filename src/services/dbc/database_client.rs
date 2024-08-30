//! # Database Client

//! This module defines the `DatabaseClient` struct, which provides a convenient and efficient interface for interacting with a Microsoft SQL Server database. 
//! The `DatabaseClient` encapsulates a connection pool and offers methods to execute SQL queries and fetch query results, streamlining database operations within the IQX Dock Manager application.


use std::str::FromStr;
use sqlx_oldapi::mssql::{MssqlPool, MssqlConnectOptions};
use sqlx_oldapi::{Error as SqlxError, query, query_as};
use std::sync::Arc;
use crate::errors::DockManagerError;

/// Represents a client for interacting with a Microsoft SQL Server database
#[derive(Debug, Clone)]
pub struct DatabaseClient {
    /// The connection pool used to manage database connections
    pub pool: Arc<MssqlPool>,
}

impl DatabaseClient {
    /// Creates a new `DatabaseClient` and establishes a connection pool to the database
    ///
    /// The connection string should be in the format `mssql://username:password@host:port/database_name`
    /// The `app_name` is used to identify the application in the database connection
    ///
    /// # Arguments
    ///
    /// * `connection_string`: The connection string to the database
    /// * `app_name`: The name of the application
    ///
    /// # Returns
    ///
    /// * `Ok(Self)`: The created `DatabaseClient` if the connection is successful
    /// * `Err(DockManagerError)`: If there's an error parsing the connection string or connecting to the database
    pub async fn new(connection_string: &str, app_name: &str) -> Result<Self, DockManagerError> {
        let mut connect_options = MssqlConnectOptions::from_str(connection_string)
            .map_err(|e| DockManagerError::DatabaseError(SqlxError::Configuration(e.into())))?;
        connect_options = connect_options.app_name(app_name);
        let pool = MssqlPool::connect_with(connect_options)
            .await
            .map_err(DockManagerError::DatabaseError)?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Executes a SQL query without returning any rows
    ///
    /// This method is useful for executing DDL (Data Definition Language) statements or DML (Data Manipulation Language) statements 
    /// that don't produce result sets (e.g., `INSERT`, `UPDATE`, `DELETE`)
    ///
    /// # Arguments
    ///
    /// * `sql_query`: The SQL query to execute
    ///
    /// # Returns
    ///
    /// * `Ok(sqlx_oldapi::mssql::MssqlQueryResult)`: If the query execution is successful
    /// * `Err(DockManagerError)`: If there's an error executing the query
    pub async fn execute<'a>(&self, sql_query: &str) -> Result<sqlx_oldapi::mssql::MssqlQueryResult, DockManagerError> {
        query(sql_query)
            .execute(&*self.pool)
            .await
            .map_err(DockManagerError::DatabaseError)
    }

    /// Executes a SQL query and fetches the results into a vector of the specified type
    ///
    /// This method is suitable for executing SELECT queries that return rows of data
    /// The type `R` must implement the `sqlx_oldapi::FromRow` trait to allow deserialization of the query results
    ///
    /// # Arguments
    ///
    /// * `query`: The SQL query to execute
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<R>)`: A vector containing the fetched rows, deserialized into the type `R`
    /// * `Err(DockManagerError)`: If there's an error executing the query or deserializing the results
    pub async fn fetch<'a, R>(&self, query: &str) -> Result<Vec<R>, DockManagerError>
        where
            R: Send + Unpin + for<'r> sqlx_oldapi::FromRow<'r, sqlx_oldapi::mssql::MssqlRow>,
    {
        query_as::<_, R>(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(DockManagerError::DatabaseError)
    }
}