//! User Layouts Module Tests

use rust_db_lib::testing_utils::{TestDataStore, TestRow, TestVal};

use super::*;
use crate::proto::google::protobuf::Empty;

#[tokio::test]
async fn test_get_user_layouts() {
    let service = UserLayoutsServiceImpl {
        data_store: TestDataStore::new(vec![
            TestRow::new(HashMap::from([
                (
                    "group_name".into(),
                    TestVal {
                        test_string: Some("List1".into()),
                        ..Default::default()
                    },
                ),
                (
                    "user_name".into(),
                    TestVal {
                        test_string: Some("User1".into()),
                        ..Default::default()
                    },
                ),
            ])),
            TestRow::new(HashMap::from([
                (
                    "group_name".into(),
                    TestVal {
                        test_string: Some("List2".into()),
                        ..Default::default()
                    },
                ),
                (
                    "user_name".into(),
                    TestVal {
                        test_string: Some("User2".into()),
                        ..Default::default()
                    },
                ),
            ])),
        ]),
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
