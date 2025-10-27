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
    fn get_str_value(&self, column_name: &str) -> String;
    fn get_i32_value(&self, column_name: &str) -> i32;
    fn get_datetime_value(&self, column_name: &str) -> chrono::DateTime<chrono::Utc>;
}

/// Abstraction for a data store capable of executing queries
#[tonic::async_trait]
pub trait DataStore<T: DataRow>: Send + Sync {
    async fn execute_query(&self, query: &str) -> Result<Vec<T>, DataStoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyRow {
        data: String,
    }
    impl DataRow for DummyRow {
        fn get_str_value(&self, _: &str) -> String {
            self.data.clone()
        }
        fn get_i32_value(&self, _: &str) -> i32 {
            self.data.len() as i32
        }
        fn get_datetime_value(&self, _: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
    }
    impl Clone for DummyRow {
        fn clone(&self) -> Self {
            DummyRow {
                data: self.data.clone(),
            }
        }
    }

    struct DummyDataStore {
        data: Vec<DummyRow>,
    }
    #[tonic::async_trait]
    impl DataStore<DummyRow> for DummyDataStore {
        async fn execute_query(&self, _: &str) -> Result<Vec<DummyRow>, DataStoreError> {
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
        let results = store.execute_query("SELECT * FROM dummy").await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_str_value("data"), "row1");
        assert_eq!(results[1].get_i32_value("data"), 4);
        assert_eq!(
            results[0].get_datetime_value("data").timestamp() <= chrono::Utc::now().timestamp(),
            true
        );
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
