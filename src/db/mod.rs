pub mod postgres;

pub trait DataRow: Send + Sync {
    fn get_str_value(&self, column_name: &str) -> String;
    fn get_i32_value(&self, column_name: &str) -> i32;
}

#[tonic::async_trait]
pub trait DataStore<T: DataRow>: Send + Sync {
    async fn execute_query(&self, query: &str) -> Vec<T>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyRow {
        data: String,
    }
    impl DataRow for DummyRow {
        fn get_str_value(&self, _column_name: &str) -> String {
            self.data.clone()
        }
        fn get_i32_value(&self, _column_name: &str) -> i32 {
            self.data.len() as i32
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
        async fn execute_query(&self, _query: &str) -> Vec<DummyRow> {
            self.data.clone()
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
        let results = store.execute_query("SELECT * FROM dummy").await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].get_str_value("data"), "row1");
        assert_eq!(results[1].get_i32_value("data"), 4);
    }
}
