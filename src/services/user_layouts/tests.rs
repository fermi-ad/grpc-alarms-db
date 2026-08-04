//! User Layouts Module Tests

use crate::proto::google::protobuf::Empty;

use super::*;
use rust_db_lib::testing_utils::{TestDataStore, TestVal};

#[derive(Clone, Debug)]
struct TestRow {
    user_name: String,
    group_name: String,
}
impl DataRow<TestVal> for TestRow {
    fn get(&self, column_name: &str) -> TestVal {
        match column_name {
            "user_name" => {
                let mut result = TestVal::new();
                result.test_string = Some(self.user_name.clone());
                result
            }
            "group_name" => {
                let mut result = TestVal::new();
                result.test_string = Some(self.group_name.clone());
                result
            }
            _ => TestVal::new(),
        }
    }
}

#[tokio::test]
async fn test_get_user_layouts() {
    let service = UserLayoutsServiceImpl {
        data_store: TestDataStore::new(vec![
            TestRow {
                group_name: "List1".to_string(),
                user_name: "User1".to_string(),
            },
            TestRow {
                group_name: "List2".to_string(),
                user_name: "User2".to_string(),
            },
        ]),
        _row_type: PhantomData,
        _val_type: PhantomData,
    };
    let result = service.get_user_layouts(Request::new(Empty {})).await;
    assert!(result.is_ok());
    let response = result
        .expect("get_user_layouts should succeed")
        .into_inner();
    assert_eq!(response.layouts.len(), 2);
    for (index, value) in response.layouts.iter().enumerate() {
        let index_text = (index + 1).to_string();
        assert_eq!(value.user_name, format!("User{index_text}"));
        assert_eq!(value.groups, vec![format!("List{index_text}")]);
    }
}
