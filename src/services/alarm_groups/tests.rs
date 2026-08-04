//! Alarm Groups Module Tests

use crate::proto::google::protobuf::{Empty, Timestamp};

use super::*;
use chrono::{DateTime, TimeZone, Utc};
use rust_db_lib::testing_utils::{TestDataStore, TestVal};
use std::vec;

#[derive(Clone)]
struct TestRow {
    group_name: String,
    description: String,
    updated_at: DateTime<Utc>,
    updated_by: String,
    group_is_user_category: bool,
    member_name: String,
    member_is_group: bool,
}
impl DataRow<TestVal> for TestRow {
    fn get(&self, column_name: &str) -> TestVal {
        match column_name {
            "description" => {
                let mut val = TestVal::new();
                val.test_string = Some(self.description.clone());
                val
            }
            "group_is_user_category" => {
                let mut val = TestVal::new();
                val.test_bool = Some(self.group_is_user_category);
                val
            }
            "group_name" => {
                let mut val = TestVal::new();
                val.test_string = Some(self.group_name.clone());
                val
            }
            "member_is_group" => {
                let mut val = TestVal::new();
                val.test_bool = Some(self.member_is_group);
                val
            }
            "member_name" => {
                let mut val = TestVal::new();
                val.test_string = Some(self.member_name.clone());
                val
            }
            "updated_at" => {
                let mut val = TestVal::new();
                val.test_datetime = Some(self.updated_at);
                val
            }
            "updated_by" => {
                let mut val = TestVal::new();
                val.test_string = Some(self.updated_by.clone());
                val
            }
            _ => TestVal::new(),
        }
    }
}

fn row1() -> TestRow {
    TestRow {
        group_name: "Group1".to_string(),
        description: "Description 1".to_string(),
        updated_at: Utc
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .single()
            .expect("Date could not be calculated"),
        updated_by: "User1".to_string(),
        group_is_user_category: false,
        member_name: "G:AMANDA1".to_string(),
        member_is_group: true,
    }
}
fn row2() -> TestRow {
    TestRow {
        group_name: "Group2".to_string(),
        description: "Description 2".to_string(),
        updated_at: Utc
            .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
            .single()
            .expect("Date could not be calculated"),
        updated_by: "User2".to_string(),
        group_is_user_category: true,
        member_name: "G:AMANDA2".to_string(),
        member_is_group: false,
    }
}

#[tokio::test]
async fn test_get_group_metadata() {
    let service = AlarmGroupsServiceImpl {
        data_store: TestDataStore::new(vec![row1(), row2()]),
        _row_type: PhantomData,
        _val_type: PhantomData,
    };
    let result = service.get_group_metadata(Request::new(Empty {})).await;
    assert!(result.is_ok());
    let response = result
        .expect("get_group_metadata should succeed")
        .into_inner();
    assert_eq!(response.metadata.len(), 2);
    for (index, value) in response.metadata.iter().enumerate() {
        let index_text = (index + 1).to_string();
        let time = Utc
            .with_ymd_and_hms(
                2024,
                1,
                (index + 1).try_into().expect("index + 1 should fit in u32"),
                0,
                0,
                0,
            )
            .single()
            .expect("Date could not be calculated");
        assert_eq!(value.name, format!("Group{index_text}"));
        assert_eq!(value.description, format!("Description {index_text}"));
        assert_eq!(
            value.updated_at,
            Some(Timestamp {
                seconds: time.timestamp(),
                nanos: time.timestamp_subsec_nanos() as i32,
            })
        );
        assert_eq!(value.updated_by, format!("User{index_text}"));
        assert_eq!(value.is_user_category, index == 1);
    }
}

#[tokio::test]
async fn test_get_groups() {
    let service = AlarmGroupsServiceImpl {
        data_store: TestDataStore::new(vec![row2()]),
        _row_type: PhantomData,
        _val_type: PhantomData,
    };
    let result = service
        .get_groups(Request::new(GroupsRequest {
            groups: vec!["Group2".to_string()],
        }))
        .await;
    assert!(result.is_ok());
    let response = result.expect("get_groups should succeed").into_inner();
    assert_eq!(response.alarm_groups.len(), 1);
    let value = response
        .alarm_groups
        .first()
        .expect("response should contain at least one group");
    let metadata = value
        .metadata
        .as_ref()
        .expect("alarm group should have metadata");
    assert_eq!(metadata.name, "Group2");
    assert_eq!(metadata.description, "Description 2");
    let time = Utc
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .expect("Date could not be calculated");
    assert_eq!(
        metadata.updated_at,
        Some(Timestamp {
            seconds: time.timestamp(),
            nanos: time.timestamp_subsec_nanos() as i32,
        })
    );
    assert_eq!(metadata.updated_by, "User2");
    assert!(metadata.is_user_category);
    assert_eq!(value.devices, vec!["G:AMANDA2"]);
    assert!(value.groups.is_empty());
}

#[tokio::test]
async fn test_get_groups_empty_request() {
    let data: Vec<TestRow> = vec![];
    let service = AlarmGroupsServiceImpl {
        data_store: TestDataStore::new(data),
        _row_type: PhantomData,
        _val_type: PhantomData,
    };
    let result = service
        .get_groups(Request::new(GroupsRequest { groups: vec![] }))
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
}
