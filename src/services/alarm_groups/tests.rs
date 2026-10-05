//! Alarm Groups Module Tests

use chrono::{TimeZone, Utc};
use rust_db_lib::testing_utils::{Operation, test_data_store};

use crate::proto::google::protobuf::{Empty, Timestamp};

use super::*;

#[tokio::test]
async fn test_get_group_metadata() {
    let time1 = Utc
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .single()
        .expect("Date could not be calculated");
    let time2 = Utc
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .expect("Date could not be calculated");
    let data_store = test_data_store!([
        [
            ("group_name", "Group1"),
            ("description", "Description 1"),
            ("updated_at", time1),
            ("updated_by", "User1"),
            ("group_is_user_category", false),
            ("member_name", "G:AMANDA1"),
            ("member_is_group", true)
        ],
        [
            ("group_name", "Group2"),
            ("description", "Description 2"),
            ("updated_at", time2),
            ("updated_by", "User2"),
            ("group_is_user_category", true),
            ("member_name", "G:AMANDA2"),
            ("member_is_group", false)
        ]
    ]);
    let service = AlarmGroupsServiceImpl::new(data_store.clone());
    let result = service.get_group_metadata(Request::new(Empty {})).await;
    assert!(result.is_ok());
    assert_eq!(
        data_store.captured_operations(),
        vec![Operation::Query(ALL_GROUPS_METADATA_QUERY.into())]
    );
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
    let time = Utc
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .expect("Date could not be calculated");
    let data_store = test_data_store!([[
        ("group_name", "Group2"),
        ("description", "Description 2"),
        ("updated_at", time),
        ("updated_by", "User2"),
        ("group_is_user_category", true),
        ("member_name", "G:AMANDA2"),
        ("member_is_group", false)
    ]]);
    let service = AlarmGroupsServiceImpl::new(data_store.clone());
    let result = service
        .get_groups(Request::new(GroupsRequest {
            groups: vec!["Group2".to_string()],
        }))
        .await;
    assert!(result.is_ok());
    let mut expected_query =
        ParameterizedQuery::new(GROUP_DETAILS_QUERY.replace("{group_name_placeholders}", "$1"));
    expected_query.bind(QueryParameter::Str("Group2".to_string()));
    assert_eq!(
        data_store.captured_operations(),
        vec![Operation::ParameterizedQuery(expected_query)]
    );
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
async fn test_get_groups_multiple_names() {
    let data_store = test_data_store!([]);
    let service = AlarmGroupsServiceImpl::new(data_store.clone());
    let result = service
        .get_groups(Request::new(GroupsRequest {
            groups: vec!["Group1".to_string(), "Group2".to_string()],
        }))
        .await;
    assert!(result.is_ok());
    assert!(
        result
            .expect("get_groups should succeed")
            .into_inner()
            .alarm_groups
            .is_empty()
    );

    let mut expected_query =
        ParameterizedQuery::new(GROUP_DETAILS_QUERY.replace("{group_name_placeholders}", "$1, $2"));
    expected_query.bind(QueryParameter::Str("Group1".to_string()));
    expected_query.bind(QueryParameter::Str("Group2".to_string()));
    assert_eq!(
        data_store.captured_operations(),
        vec![Operation::ParameterizedQuery(expected_query)]
    );
}

#[tokio::test]
async fn test_get_groups_empty_request() {
    let data_store = test_data_store!([]);
    let service = AlarmGroupsServiceImpl::new(data_store.clone());
    let result = service
        .get_groups(Request::new(GroupsRequest { groups: vec![] }))
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    assert!(data_store.captured_operations().is_empty());
}
