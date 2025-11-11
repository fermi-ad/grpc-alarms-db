pub mod postgres;
use std::{error::Error, fmt::Display};
use tonic::Status;

/// Custom error type for DataStore operations
#[derive(Debug)]
pub struct DataStoreError {
    details: String,
}
impl Display for DataStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DataStoreError: {}", self.details)
    }
}
impl Error for DataStoreError {}

impl From<DataStoreError> for Status {
    fn from(_: DataStoreError) -> Self {
        Status::internal(
            "An error occured while accessing the data store. See system logs for details.",
        )
    }
}

/// Abstraction representing a single row retrieved from a data store
pub trait DataRow: Send + Sync {
    fn get_bool_value(&self, column_name: &str) -> bool;
    fn get_datetime_value(&self, column_name: &str) -> chrono::DateTime<chrono::Utc>;
    fn get_str_value(&self, column_name: &str) -> String;
}

/// Abstraction for a data store capable of executing queries
#[tonic::async_trait]
pub trait DataStore<T: DataRow>: Clone + Send + Sync {
    /// Executes a basic SQL statement. If any user input is required, use [`execute_parameterized_query()`]
    async fn execute_query(&self, query: String) -> Result<Vec<T>, DataStoreError>;

    /// Executes a parameterized query. It is expected that the query string will have sequential placeholders
    /// for the paramterized data.
    ///
    /// Example: If passing 5 elements into the query. the query should contain $1, $2, $3, $4, and $5, and
    /// the bindings should have no fewer than 5 elements (any additional elements beyond 5 will be ignored).
    async fn execute_parameterized_query(
        &self,
        query: String,
        bindings: Vec<String>,
    ) -> Result<Vec<T>, DataStoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyRow {
        data: String,
    }
    impl DataRow for DummyRow {
        fn get_bool_value(&self, _: &str) -> bool {
            false
        }
        fn get_datetime_value(&self, _: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
        fn get_str_value(&self, _: &str) -> String {
            self.data.clone()
        }
    }
    impl Clone for DummyRow {
        fn clone(&self) -> Self {
            DummyRow {
                data: self.data.clone(),
            }
        }
    }
    impl PartialEq for DummyRow {
        fn eq(&self, other: &Self) -> bool {
            self.data == other.data
        }
    }

    struct DummyDataStore {
        data: Vec<DummyRow>,
    }
    impl Clone for DummyDataStore {
        fn clone(&self) -> Self {
            Self {
                data: self.data.clone(),
            }
        }
    }
    #[tonic::async_trait]
    impl DataStore<DummyRow> for DummyDataStore {
        async fn execute_query(&self, _: String) -> Result<Vec<DummyRow>, DataStoreError> {
            Ok(self.data.clone())
        }
        async fn execute_parameterized_query(
            &self,
            _: String,
            _: Vec<String>,
        ) -> Result<Vec<DummyRow>, DataStoreError> {
            Ok(self.data.clone())
        }
    }

    #[tokio::test]
    async fn test_dummy_data_store() {
        let data1 = DummyRow {
            data: "row1".to_string(),
        };
        let data2 = DummyRow {
            data: "row2".to_string(),
        };
        let store = DummyDataStore {
            data: vec![data1, data2],
        };
        let results = store
            .execute_query("SELECT * FROM dummy".to_string())
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_str_value("data"), "row1");
        assert_eq!(
            results[0].get_datetime_value("data").timestamp() <= chrono::Utc::now().timestamp(),
            true
        );
        assert_eq!(results[0].get_bool_value("data"), false);

        let parameterized_results = store
            .execute_parameterized_query(
                "SELECT * FROM dummy".to_string(),
                vec!["Some binding".to_string()],
            )
            .await
            .unwrap();
        assert_eq!(parameterized_results, results);
    }

    #[test]
    fn test_display_datastore_error() {
        let error = DataStoreError {
            details: "Test error".to_string(),
        };
        assert_eq!(format!("{}", error), "DataStoreError: Test error");
    }

    #[test]
    fn test_datastore_error_to_status() {
        let error = DataStoreError {
            details: "Test error".to_string(),
        };
        let status: Status = error.into();
        assert_eq!(status.code(), tonic::Code::Internal);
    }
}
