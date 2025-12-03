use crate::db::{DataRow, DataStore, DataStoreError};
use sqlx::{
    Pool, Postgres, Row,
    postgres::{PgPoolOptions, PgRow},
};
use std::{env, time::Duration};

/// Postgres implementation of the DataStore trait
pub struct PostgresDataStore {
    db_pool: Pool<Postgres>,
}

async fn establish_connection_pool() -> Pool<Postgres> {
    let db_connect = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_connect)
        .await
        .expect("Failed to connect to database...")
}

impl PostgresDataStore {
    pub async fn new() -> Self {
        PostgresDataStore {
            db_pool: establish_connection_pool().await,
        }
    }
}

impl Clone for PostgresDataStore {
    fn clone(&self) -> Self {
        Self {
            db_pool: self.db_pool.clone(),
        }
    }
}

/// Represents a single row retrieved from a Postgres database
/// implementing the DataRow trait. In this case, it wraps sqlx::PgRow
/// to provide the necessary methods.
pub struct PostgresDataRow {
    row: PgRow,
}
impl From<PgRow> for PostgresDataRow {
    fn from(row: PgRow) -> Self {
        Self { row }
    }
}
impl DataRow for PostgresDataRow {
    fn get_bool_value(&self, column_name: &str) -> bool {
        self.row.get(column_name)
    }
    fn get_datetime_value(&self, column_name: &str) -> chrono::DateTime<chrono::Utc> {
        self.row.get(column_name)
    }
    fn get_str_value(&self, column_name: &str) -> String {
        self.row.get(column_name)
    }
}

/// Encapsulates the execution of queries against a Postgres database;
/// Returns results as PostgresDataRow instances.
#[tonic::async_trait]
impl DataStore<PostgresDataRow> for PostgresDataStore {
    async fn execute_query(&self, query: String) -> Result<Vec<PostgresDataRow>, DataStoreError> {
        let query_result = sqlx::query(query.as_str()).fetch_all(&self.db_pool).await;
        match query_result {
            Ok(rows) => Ok(rows.into_iter().map(PostgresDataRow::from).collect()),
            Err(e) => {
                tracing::error!("Alarm list retrieval query failed: {}", e);
                Err(DataStoreError {
                    details: "Query execution failed. See system logs for details.".to_string(),
                })
            }
        }
    }
    async fn execute_parameterized_query(
        &self,
        query: String,
        bindings: Vec<String>,
    ) -> Result<Vec<PostgresDataRow>, DataStoreError> {
        let mut query_builder = sqlx::query(query.as_str());
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }
        let query_result = query_builder.fetch_all(&self.db_pool).await;
        match query_result {
            Ok(rows) => Ok(rows.into_iter().map(PostgresDataRow::from).collect()),
            Err(e) => {
                tracing::error!("Alarm list retrieval query failed: {}", e);
                Err(DataStoreError {
                    details: "Query execution failed. See system logs for details.".to_string(),
                })
            }
        }
    }
}
