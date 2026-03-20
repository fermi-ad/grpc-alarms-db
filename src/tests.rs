//! Main Module Tests

use super::*;
use rust_db_lib::testing_utils::{TestDataStore, TestVal};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone, Debug)]
struct TestRow;
impl DataRow<TestVal> for TestRow {
    fn get(&self, _: &str) -> TestVal {
        TestVal::new()
    }
}

#[tokio::test]
async fn test_start_server() {
    let data_store = TestDataStore::new(Vec::<TestRow>::new());
    let future = start_server(data_store);
    let result = timeout(Duration::from_secs(1), future).await;
    assert!(result.is_err());
}

#[test]
fn test_row_get() {
    assert_eq!(TestVal::new(), TestRow.get(""));
}
