use std::{cmp::Ordering, collections::HashMap, marker::PhantomData};

use tonic::{Request, Response, Status};
use tracing::info;

use crate::db::{DataRow, DataStore, DataStoreError};

pub mod proto {
    tonic::include_proto!("services.alarm_groups");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("alarmprotos_descriptor");
}
use proto::{
    AlarmGroup, AlarmGroupMetadata, AlarmGroupMetadatum, AlarmGroups, GroupsRequest,
    alarm_group_service_server::AlarmGroupService,
};

/// A service wrapping a DataStore to provide alarm group information, and implementing the Protobuf-defined gRPC service.
pub struct AlarmGroupsServiceImpl<T: DataRow, U: DataStore<T>> {
    data_store: Box<U>,
    _row_type: PhantomData<T>,
}

impl<T: DataRow, U: DataStore<T>> AlarmGroupsServiceImpl<T, U> {
    pub fn new(data_store: Box<U>) -> Self {
        Self {
            data_store,
            _row_type: PhantomData,
        }
    }

    fn convert_datetime_to_timestamp(
        datetime: chrono::DateTime<chrono::Utc>,
    ) -> Option<prost_types::Timestamp> {
        Some(prost_types::Timestamp {
            seconds: datetime.timestamp(),
            nanos: datetime.timestamp_subsec_nanos() as i32,
        })
    }

    fn create_metadatum(row: &T, name: String) -> AlarmGroupMetadatum {
        AlarmGroupMetadatum {
            name,
            description: row.get_str_value("description"),
            updated_at: Self::convert_datetime_to_timestamp(row.get_datetime_value("updated_at")),
            updated_by: row.get_str_value("updated_by"),
            is_user_category: row.get_bool_value("group_is_user_category"),
        }
    }

    fn rows_to_metadata(rows: Vec<T>) -> Vec<AlarmGroupMetadatum> {
        rows.into_iter()
            .map(|row| Self::create_metadatum(&row, row.get_str_value("group_name")))
            .collect()
    }

    /// Retrieves all alarm group metadata.
    async fn get_all_metadata(&self) -> Result<Vec<AlarmGroupMetadatum>, DataStoreError> {
        info!("Query for all alarm group metadata ");

        let alarm_group_query = String::from(
            "
            SELECT 
                g.group_name,
                g.description,
                g.updated_at,
                g.updated_by,
                EXISTS (
                    SELECT
                    FROM alarmsapp.user_layouts u
                    WHERE g.group_name = u.group_name
                ) AS group_is_user_category
            FROM 
                alarmsapp.groups g
            ORDER BY
                updated_by,
                group_name
            ;
        ",
        );
        let query_result = self.data_store.execute_query(alarm_group_query).await?;
        let mut metadata = Self::rows_to_metadata(query_result);
        metadata.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(metadata)
    }

    fn rows_to_groups(rows: Vec<T>) -> Vec<AlarmGroup> {
        rows.into_iter()
            .fold(
                HashMap::new(),
                |mut accumulator: HashMap<String, AlarmGroup>, row: T| {
                    let group_name = row.get_str_value("group_name");
                    let alarm_group =
                        accumulator
                            .entry(group_name.clone())
                            .or_insert_with(|| AlarmGroup {
                                metadata: Some(Self::create_metadatum(&row, group_name)),
                                devices: Vec::new(),
                                groups: Vec::new(),
                            });
                    let member_name = row.get_str_value("member_name");
                    if row.get_bool_value("member_is_group") {
                        alarm_group.groups.push(member_name);
                    } else {
                        alarm_group.devices.push(member_name);
                    }
                    accumulator
                },
            )
            .into_values()
            .collect()
    }

    fn sort_groups(a: &AlarmGroup, b: &AlarmGroup) -> Ordering {
        match a.metadata.as_ref() {
            Some(group) => match b.metadata.as_ref() {
                Some(other_group) => group.name.cmp(&other_group.name),
                None => Ordering::Less,
            },
            None => Ordering::Greater,
        }
    }

    async fn get_requested_groups(
        &self,
        specified_groups: Vec<String>,
    ) -> Result<Vec<AlarmGroup>, DataStoreError> {
        info!(
            "Query for alarm groups {:?}, and associated devices ",
            specified_groups
        );

        let needed_placeholders: Vec<String> = (1..specified_groups.len())
            .map(|index| format!("${}", index))
            .collect();

        // Uses a recursive Common Table Expression (CTE) to build the result rows down to the device level.
        // That is, users can specify the top-level groups they want, and this SQL query will return all the
        // child, grandchild, etc. objects under it.
        let alarm_group_query = format!(
            "
            WITH RECURSIVE members AS (
                SELECT 
                    group_name,
                    member_name,
                    member_is_group
                FROM 
                    alarmsapp.group_membership
                WHERE
                    group_name IN ({})
                UNION ALL
                SELECT
                    gm.group_name,
                    gm.member_name,
                    gm.member_is_group,
                FROM 
                    alarmsapp.group_membership gm 
                    INNER JOIN members m 
                        ON gm.group_name = m.member_name
            )
            SELECT 
                g.group_name,
                g.description,
                g.updated_at,
                g.updated_by,
                EXISTS (
                    SELECT 
                    FROM alarmsapp.user_layouts u
                    WHERE g.group_name = u.group_name
                ) AS group_is_user_category,
                m.member_name,
                m.member_is_group
            FROM
                alarmsapp.groups g
                INNER JOIN members m
                    ON g.group_name = m.group_name
            ORDER BY 
                g.group_name, 
                m.member_name
            ;
            ",
            needed_placeholders.join(", ")
        );

        let query_result = self
            .data_store
            .execute_parameterized_query(alarm_group_query, specified_groups)
            .await?;
        let mut groups = Self::rows_to_groups(query_result);
        groups.sort_by(Self::sort_groups);

        Ok(groups)
    }
}

/// Implements the AlarmGroupService gRPC service.
///
/// Translates query results from the DataStore into gRPC AlarmGroup messages.
#[tonic::async_trait]
impl<T: DataRow + 'static, U: DataStore<T> + 'static> AlarmGroupService
    for AlarmGroupsServiceImpl<T, U>
{
    async fn get_group_metadata(
        &self,
        _: Request<()>,
    ) -> Result<Response<AlarmGroupMetadata>, Status> {
        let metadata: Vec<AlarmGroupMetadatum> = self.get_all_metadata().await?;

        Ok(Response::new(AlarmGroupMetadata { metadata }))
    }

    async fn get_groups(
        &self,
        request: Request<GroupsRequest>,
    ) -> Result<Response<AlarmGroups>, Status> {
        let requested_groups = request.into_inner().groups;
        let alarm_groups: Vec<AlarmGroup> = self.get_requested_groups(requested_groups).await?;

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
        updated_at: chrono::DateTime<chrono::Utc>,
        updated_by: String,
        group_is_user_category: bool,
        member_name: String,
        member_is_group: bool,
    }
    impl Clone for TestRow {
        fn clone(&self) -> Self {
            Self {
                group_name: self.group_name.clone(),
                description: self.description.clone(),
                updated_at: self.updated_at.clone(),
                updated_by: self.updated_by.clone(),
                group_is_user_category: self.group_is_user_category.clone(),
                member_name: self.member_name.clone(),
                member_is_group: self.member_is_group.clone(),
            }
        }
    }
    impl DataRow for TestRow {
        fn get_bool_value(&self, column_name: &str) -> bool {
            match column_name {
                "group_is_user_category" => self.group_is_user_category,
                "member_is_group" => self.member_is_group,
                _ => false,
            }
        }
        fn get_datetime_value(&self, column_name: &str) -> chrono::DateTime<chrono::Utc> {
            match column_name {
                "updated_at" => self.updated_at,
                _ => chrono::Utc::now(),
            }
        }
        fn get_str_value(&self, column_name: &str) -> String {
            match column_name {
                "group_name" => self.group_name.clone(),
                "description" => self.description.clone(),
                "member_name" => self.member_name.clone(),
                "updated_by" => self.updated_by.clone(),
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
                    updated_at: chrono::Utc
                        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                    updated_by: "User1".to_string(),
                    group_is_user_category: false,
                    member_name: "G:AMANDA1".to_string(),
                    member_is_group: true,
                },
                row2: TestRow {
                    group_name: "Group2".to_string(),
                    description: "Description 2".to_string(),
                    updated_at: chrono::Utc
                        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                    updated_by: "User2".to_string(),
                    group_is_user_category: true,
                    member_name: "G:AMANDA2".to_string(),
                    member_is_group: false,
                },
            }
        }
    }
    impl Clone for TestDataStore {
        fn clone(&self) -> Self {
            Self {
                row1: self.row1.clone(),
                row2: self.row2.clone(),
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
    async fn test_get_group_metadata() {
        let service = AlarmGroupsServiceImpl {
            data_store: Box::new(TestDataStore::new()),
            _row_type: PhantomData,
        };
        let result = service.get_group_metadata(Request::new(())).await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert_eq!(response.metadata.len(), 2);
        for (index, value) in response.metadata.iter().enumerate() {
            let index_text = (index + 1).to_string();
            let time = chrono::Utc
                .with_ymd_and_hms(2024, 1, (index + 1).try_into().unwrap(), 0, 0, 0)
                .single()
                .expect("Date could not be calculated");
            assert_eq!(value.name, format!("Group{}", index_text));
            assert_eq!(value.description, format!("Description {}", index_text));
            assert_eq!(
                value.updated_at,
                Some(prost_types::Timestamp {
                    seconds: time.timestamp(),
                    nanos: time.timestamp_subsec_nanos() as i32,
                })
            );
            assert_eq!(value.updated_by, format!("User{}", index_text));
            assert_eq!(value.is_user_category, index == 1);
        }
    }

    #[tokio::test]
    async fn test_get_groups() {
        let service = AlarmGroupsServiceImpl {
            data_store: Box::new(TestDataStore::new()),
            _row_type: PhantomData,
        };
        let result = service
            .get_groups(Request::new(GroupsRequest {
                groups: vec!["Group2".to_string()],
            }))
            .await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert_eq!(response.alarm_groups.len(), 1);
        let value = response.alarm_groups.first().unwrap();
        let metadata = value.metadata.as_ref().unwrap();
        assert_eq!(metadata.name, "Group2");
        assert_eq!(metadata.description, "Description 2");
        let time = chrono::Utc
            .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
            .single()
            .expect("Date could not be calculated");
        assert_eq!(
            metadata.updated_at,
            Some(prost_types::Timestamp {
                seconds: time.timestamp(),
                nanos: time.timestamp_subsec_nanos() as i32,
            })
        );
        assert_eq!(metadata.updated_by, "User2");
        assert!(metadata.is_user_category);
        assert_eq!(value.devices, vec!["G:AMANDA2"]);
        assert!(value.groups.is_empty());
    }
}
