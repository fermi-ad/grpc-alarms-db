//! User Layouts Module Tests

use rust_db_lib::testing_utils::{Operation, test_data_store};

use super::*;
use crate::proto::google::protobuf::Empty;

#[tokio::test]
async fn test_get_user_layouts() {
    let data_store = test_data_store!([
        [("group_name", "List1"), ("user_name", "User1")],
        [("group_name", "List2"), ("user_name", "User2")]
    ]);
    let service = UserLayoutsServiceImpl::new(data_store.clone());
    let result = service.get_user_layouts(Request::new(Empty {})).await;
    assert!(result.is_ok());
    assert_eq!(
        data_store.captured_operations(),
        vec![Operation::Query(GET_ALL_LAYOUTS_QUERY.into())]
    );
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
