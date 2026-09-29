//! Alarm Groups Module Tests

use std::vec;

use chrono::{TimeZone, Utc};
use rust_db_lib::testing_utils::{TestDataStore, TestRow, TestVal};

use crate::proto::google::protobuf::{Empty, Timestamp};

use super::*;

fn row1() -> TestRow {
    TestRow::new(HashMap::from([
        (
            "group_name".into(),
            TestVal {
                test_string: Some("Group1".into()),
                ..Default::default()
            },
        ),
        (
            "description".into(),
            TestVal {
                test_string: Some("Description 1".into()),
                ..Default::default()
            },
        ),
        (
            "updated_at".into(),
            TestVal {
                test_datetime: Some(
                    Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                ),
                ..Default::default()
            },
        ),
        (
            "updated_by".into(),
            TestVal {
                test_string: Some("User1".into()),
                ..Default::default()
            },
        ),
        (
            "group_is_user_category".into(),
            TestVal {
                test_bool: Some(false),
                ..Default::default()
            },
        ),
        (
            "member_name".into(),
            TestVal {
                test_string: Some("G:AMANDA1".into()),
                ..Default::default()
            },
        ),
        (
            "member_is_group".into(),
            TestVal {
                test_bool: Some(true),
                ..Default::default()
            },
        ),
    ]))
}

fn row2() -> TestRow {
    TestRow::new(HashMap::from([
        (
            "group_name".into(),
            TestVal {
                test_string: Some("Group2".into()),
                ..Default::default()
            },
        ),
        (
            "description".into(),
            TestVal {
                test_string: Some("Description 2".into()),
                ..Default::default()
            },
        ),
        (
            "updated_at".into(),
            TestVal {
                test_datetime: Some(
                    Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                ),
                ..Default::default()
            },
        ),
        (
            "updated_by".into(),
            TestVal {
                test_string: Some("User2".into()),
                ..Default::default()
            },
        ),
        (
            "group_is_user_category".into(),
            TestVal {
                test_bool: Some(true),
                ..Default::default()
            },
        ),
        (
            "member_name".into(),
            TestVal {
                test_string: Some("G:AMANDA2".into()),
                ..Default::default()
            },
        ),
        (
            "member_is_group".into(),
            TestVal {
                test_bool: Some(false),
                ..Default::default()
            },
        ),
    ]))
}

#[tokio::test]
async fn test_get_group_metadata() {
    let service = AlarmGroupsServiceImpl::new(TestDataStore::new(vec![row1(), row2()]));
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
    let service = AlarmGroupsServiceImpl::new(TestDataStore::new(vec![row2()]));
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
    let service = AlarmGroupsServiceImpl::new(TestDataStore::new(vec![]));
    let result = service
        .get_groups(Request::new(GroupsRequest { groups: vec![] }))
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
}
