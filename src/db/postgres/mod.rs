use crate::db::{DataRow, DataStore};
use sqlx::{
    Pool, Postgres, Row,
    postgres::{PgPoolOptions, PgRow},
};
use std::{env, time::Duration};
use tonic::Status;

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
        PostgresDataStore {
            db_pool: self.db_pool.clone(),
        }
    }
}

pub struct PostgresDataRow {
    row: PgRow,
}
impl DataRow for PostgresDataRow {
    fn get_str_value(&self, column_name: &str) -> String {
        self.row.get(column_name)
    }
    fn get_i32_value(&self, column_name: &str) -> i32 {
        self.row.get(column_name)
    }
}

#[tonic::async_trait]
impl DataStore<PostgresDataRow> for PostgresDataStore {
    async fn execute_query(&self, query: &str) -> Vec<PostgresDataRow> {
        let sql_rows = sqlx::query(query)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| Status::internal(format!("Alarm list retrieval query failed: {}", e)));
        match sql_rows {
            Err(_) => Vec::new(),
            Ok(rows) => rows
                .into_iter()
                .map(|row| PostgresDataRow { row })
                .collect(),
        }
    }
}
