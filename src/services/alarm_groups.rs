use std::collections::HashMap;

use tonic::{Request, Response, Status};
use tracing::info;

use crate::db::{DataRow, DataStore, DataStoreError};

pub mod proto {
    tonic::include_proto!("services.alarm_groups");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("alarmprotos_descriptor");
}
use proto::{
    AlarmGroup, AlarmGroups, GroupsRequest, alarm_group_service_server::AlarmGroupService,
};

/// A service wrapping a DataStore to provide alarm group information, and implementing the Protobuf-defined gRPC service.
pub struct AlarmGroupsServiceImpl<T: DataRow> {
    data_store: Box<dyn DataStore<T>>,
}

impl<T: DataRow> AlarmGroupsServiceImpl<T> {
    pub fn new(data_store: Box<dyn DataStore<T>>) -> Self {
        Self { data_store }
    }

    fn construct_query(where_clause: Option<String>) -> String {
        format!(
            "
            SELECT 
                g.group_name,
                g.description,
                g.modified_date,
                g.modified_user,
                m.member_name,
                m.member_is_group
            FROM 
                alarms_application.groups g
                INNER JOIN
                    alarms_application.group_membership m
                    ON
                        g.group_name = m.group_name
            {}
            ORDER BY 
                g.group_name, 
                m.member_name
            ;
            ",
            where_clause.unwrap_or_default()
        )
    }

    fn convert_datetime_to_timestamp(
        datetime: chrono::DateTime<chrono::Utc>,
    ) -> Option<prost_types::Timestamp> {
        Some(prost_types::Timestamp {
            seconds: datetime.timestamp(),
            nanos: datetime.timestamp_subsec_nanos() as i32,
        })
    }

    fn process_query_result(rows: Vec<T>) -> Vec<AlarmGroup> {
        let mut sortable_rows = rows
            .into_iter()
            .fold(
                HashMap::new(),
                |mut acc: HashMap<String, AlarmGroup>, row| {
                    let group_name = row.get_str_value("group_name");
                    let alarm_group = acc.entry(group_name.clone()).or_insert(AlarmGroup {
                        name: group_name,
                        description: row.get_str_value("description"),
                        modified_date: Self::convert_datetime_to_timestamp(
                            row.get_datetime_value("modified_date"),
                        ),
                        modified_user: row.get_str_value("modified_user"),
                        devices: Vec::new(),
                        groups: Vec::new(),
                    });
                    let member_name = row.get_str_value("member_name");
                    if row.get_bool_value("member_is_group") {
                        alarm_group.groups.push(member_name);
                    } else {
                        alarm_group.devices.push(member_name);
                    }
                    acc
                },
            )
            .into_values()
            .collect::<Vec<AlarmGroup>>();
        sortable_rows.sort_by(|a, b| a.name.cmp(&b.name));
        sortable_rows
    }

    /// Retrieves all devices and their associated alarm group.
    async fn run_full_query(&self) -> Result<Vec<AlarmGroup>, DataStoreError> {
        info!("Query for all alarm groups and associated devices ");

        let alarm_group_query = Self::construct_query(None);

        let query_result = self.data_store.execute_query(alarm_group_query).await?;
        let alarm_groups = Self::process_query_result(query_result);
        Ok(alarm_groups)
    }

    async fn run_parameterized_query(
        &self,
        specified_groups: Vec<String>,
    ) -> Result<Vec<AlarmGroup>, DataStoreError> {
        info!(
            "Query for alarm groups [{:?}], and associated devices ",
            specified_groups
        );

        let needed_placeholders: Vec<String> = (1..specified_groups.len())
            .map(|index| format!("${}", index))
            .collect();
        let where_clause = format!("WHERE group_name IN ({})", needed_placeholders.join(", "));
        let alarm_group_query = Self::construct_query(Some(where_clause));

        let query_result = self
            .data_store
            .execute_parameterized_query(alarm_group_query, specified_groups)
            .await?;
        let alarm_groups = Self::process_query_result(query_result);
        Ok(alarm_groups)
    }
}

/// Implements the AlarmGroupService gRPC service.
///
/// Translates query results from the DataStore into gRPC AlarmGroup messages.
#[tonic::async_trait]
impl<T: DataRow + 'static> AlarmGroupService for AlarmGroupsServiceImpl<T> {
    async fn get_all_groups(&self, _: Request<()>) -> Result<Response<AlarmGroups>, Status> {
        let alarm_groups: Vec<AlarmGroup> = self.run_full_query().await?;
        Ok(Response::new(AlarmGroups { alarm_groups }))
    }

    async fn get_specified_groups(
        &self,
        request: Request<GroupsRequest>,
    ) -> Result<Response<AlarmGroups>, Status> {
        let requested_groups = request.into_inner().groups;
        let alarm_groups: Vec<AlarmGroup> = self.run_parameterized_query(requested_groups).await?;
        Ok(Response::new(AlarmGroups { alarm_groups }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    struct TestRow {
        group_name: String,
        description: String,
        modified_date: chrono::DateTime<chrono::Utc>,
        modified_user: String,
        member_name: String,
        member_is_group: bool,
    }
    impl Clone for TestRow {
        fn clone(&self) -> Self {
            Self {
                group_name: self.group_name.clone(),
                description: self.description.clone(),
                modified_date: self.modified_date.clone(),
                modified_user: self.modified_user.clone(),
                member_name: self.member_name.clone(),
                member_is_group: self.member_is_group.clone(),
            }
        }
    }
    impl DataRow for TestRow {
        fn get_bool_value(&self, column_name: &str) -> bool {
            match column_name {
                "member_is_group" => self.member_is_group,
                _ => false,
            }
        }
        fn get_datetime_value(&self, column_name: &str) -> chrono::DateTime<chrono::Utc> {
            match column_name {
                "modified_date" => self.modified_date,
                _ => chrono::Utc::now(),
            }
        }
        fn get_str_value(&self, column_name: &str) -> String {
            match column_name {
                "group_name" => self.group_name.clone(),
                "description" => self.description.clone(),
                "member_name" => self.member_name.clone(),
                "modified_user" => self.modified_user.clone(),
                _ => "".to_string(),
            }
        }
    }

    struct TestDataStore {
        row1: TestRow,
        row2: TestRow,
    }
    impl TestDataStore {
        pub fn new() -> Self {
            Self {
                row1: TestRow {
                    group_name: "Group1".to_string(),
                    description: "Description 1".to_string(),
                    modified_date: chrono::Utc
                        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                    modified_user: "User1".to_string(),
                    member_name: "G:AMANDA1".to_string(),
                    member_is_group: true,
                },
                row2: TestRow {
                    group_name: "Group2".to_string(),
                    description: "Description 2".to_string(),
                    modified_date: chrono::Utc
                        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                    modified_user: "User2".to_string(),
                    member_name: "G:AMANDA2".to_string(),
                    member_is_group: false,
                },
            }
        }
    }
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _: String) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![self.row1.clone(), self.row2.clone()])
        }
        async fn execute_parameterized_query(
            &self,
            _: String,
            _: Vec<String>,
        ) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![self.row2.clone()])
        }
    }

    #[tokio::test]
    async fn test_get_all_groups() {
        let service = AlarmGroupsServiceImpl {
            data_store: Box::new(TestDataStore::new()),
        };
        let result = service.get_all_groups(Request::new(())).await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert_eq!(response.alarm_groups.len(), 2);
        for (index, value) in response.alarm_groups.iter().enumerate() {
            let index_text = (index + 1).to_string();
            let time = chrono::Utc
                .with_ymd_and_hms(2024, 1, (index + 1).try_into().unwrap(), 0, 0, 0)
                .single()
                .expect("Date could not be calculated");
            assert_eq!(value.name, format!("Group{}", index_text));
            assert_eq!(value.description, format!("Description {}", index_text));
            assert_eq!(
                value.modified_date,
                Some(prost_types::Timestamp {
                    seconds: time.timestamp(),
                    nanos: time.timestamp_subsec_nanos() as i32,
                })
            );
            assert_eq!(value.modified_user, format!("User{}", index_text));
            if value.devices.len() > 0 {
                assert_eq!(value.devices, vec![format!("G:AMANDA{}", index_text)]);
            }
            if value.groups.len() > 0 {
                assert_eq!(value.groups, vec![format!("G:AMANDA{}", index_text)]);
            }
        }
    }

    #[tokio::test]
    async fn test_get_specified_groups() {
        let service = AlarmGroupsServiceImpl {
            data_store: Box::new(TestDataStore::new()),
        };
        let result = service
            .get_specified_groups(Request::new(GroupsRequest {
                groups: vec!["Group2".to_string()],
            }))
            .await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert_eq!(response.alarm_groups.len(), 1);
        let value = response.alarm_groups.first().unwrap();
        assert_eq!(value.name, "Group2");
        assert_eq!(value.description, "Description 2");
        let time = chrono::Utc
            .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
            .single()
            .expect("Date could not be calculated");
        assert_eq!(
            value.modified_date,
            Some(prost_types::Timestamp {
                seconds: time.timestamp(),
                nanos: time.timestamp_subsec_nanos() as i32,
            })
        );
        assert_eq!(value.modified_user, "User2");
        assert_eq!(value.devices, vec!["G:AMANDA2"]);
        assert!(value.groups.is_empty());
    }
}
