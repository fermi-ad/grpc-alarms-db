use std::collections::HashMap;

use tonic::{Request, Response, Status};
use tracing::info;

use proto::{AlarmList, AlarmLists, EmptyRequest, alarm_list_service_server::AlarmListService};

use crate::db::{DataRow, DataStore, DataStoreError};

pub mod proto {
    tonic::include_proto!("alarmlists");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("alarmprotos_descriptor");
}

pub struct AlarmListServiceImpl<T: DataRow> {
    data_store: Box<dyn DataStore<T>>,
}

impl<T: DataRow> AlarmListServiceImpl<T> {
    pub fn new(data_store: Box<dyn DataStore<T>>) -> Self {
        Self { data_store }
    }

    fn convert_datetime_to_timestamp(
        &self,
        datetime: chrono::DateTime<chrono::Utc>,
    ) -> Option<prost_types::Timestamp> {
        Some(prost_types::Timestamp {
            seconds: datetime.timestamp(),
            nanos: datetime.timestamp_subsec_nanos() as i32,
        })
    }

    async fn generate_naive_alarm_lists(&self) -> Result<HashMap<i32, AlarmList>, DataStoreError> {
        info!("Query for alarm lists ");

        let alarm_list_query: &str = "
            SELECT l.list_number, l.name AS list_name, l.long_name, l.description, l.modify_date, l.modify_user_name, d.name AS device_name
            FROM hendricks.alarm_list_info l, accdb.device d
            WHERE d.alarm_list_id = l.list_number
            ORDER BY l.list_number, d.name;
            ";

        let rows = self.data_store.execute_query(alarm_list_query).await?;
        Ok(rows
            .into_iter()
            .fold(HashMap::new(), |mut acc: HashMap<i32, AlarmList>, row| {
                let list_number = row.get_i32_value("list_number");
                let alarm_list = acc.entry(list_number).or_insert(AlarmList {
                    list_number,
                    name: row.get_str_value("list_name"),
                    long_name: row.get_str_value("long_name"),
                    description: row.get_str_value("description"),
                    modify_date: self
                        .convert_datetime_to_timestamp(row.get_datetime_value("modify_date")),
                    modify_user_name: row.get_str_value("modify_user_name"),
                    member_devices: Vec::new(),
                });
                let device_name = row.get_str_value("device_name");
                alarm_list.member_devices.push(device_name);
                acc
            }))
    }

    async fn assign_devices_to_node_lists(
        &self,
        naive_alarm_lists: &mut HashMap<i32, AlarmList>,
    ) -> Result<(), DataStoreError> {
        let node_alarm_list_query = "
            SELECT n.list_number, d.device_name
            FROM accdb.device d
            OUTER JOIN hendricks.alarm_list_nodes n ON d.trunk = n.trunk AND d.node = n.node
            WHERE d.alarm_list_id = 0 AND n.list_number <> 0
        ";
        let rows = self.data_store.execute_query(node_alarm_list_query).await?;
        for row in rows {
            let list_number = row.get_i32_value("list_number");
            if let Some(alarm_list) = naive_alarm_lists.get_mut(&list_number) {
                let device_name = row.get_str_value("device_name");
                alarm_list.member_devices.push(device_name);
            }
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl<T: DataRow + 'static> AlarmListService for AlarmListServiceImpl<T> {
    async fn get_alarm_lists(
        &self,
        _: Request<EmptyRequest>,
    ) -> Result<Response<AlarmLists>, Status> {
        let mut alarm_lists: HashMap<i32, AlarmList> = self.generate_naive_alarm_lists().await?;
        self.assign_devices_to_node_lists(&mut alarm_lists).await?;
        let mut sorted_results = alarm_lists.into_values().collect::<Vec<AlarmList>>();
        sorted_results.sort_by(|a, b| a.list_number.cmp(&b.list_number));
        Ok(Response::new(AlarmLists {
            alarm_lists: sorted_results,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    struct TestRow {
        list_name: String,
        long_name: String,
        description: String,
        device_name: String,
        modify_date: chrono::DateTime<chrono::Utc>,
        modify_user_name: String,
        list_number: i32,
    }
    impl DataRow for TestRow {
        fn get_str_value(&self, column_name: &str) -> String {
            match column_name {
                "list_name" => self.list_name.clone(),
                "long_name" => self.long_name.clone(),
                "description" => self.description.clone(),
                "device_name" => self.device_name.clone(),
                "modify_user_name" => self.modify_user_name.clone(),
                _ => "".to_string(),
            }
        }
        fn get_i32_value(&self, column_name: &str) -> i32 {
            match column_name {
                "list_number" => self.list_number,
                _ => 0,
            }
        }
        fn get_datetime_value(&self, column_name: &str) -> chrono::DateTime<chrono::Utc> {
            match column_name {
                "modify_date" => self.modify_date,
                _ => chrono::Utc::now(),
            }
        }
    }

    struct TestDataStore;
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _: &str) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![
                TestRow {
                    list_name: "List1".to_string(),
                    long_name: "Long List 1".to_string(),
                    description: "Description 1".to_string(),
                    device_name: "G:AMANDA1".to_string(),
                    modify_date: chrono::Utc
                        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                    modify_user_name: "User1".to_string(),
                    list_number: 1,
                },
                TestRow {
                    list_name: "List2".to_string(),
                    long_name: "Long List 2".to_string(),
                    description: "Description 2".to_string(),
                    device_name: "G:AMANDA2".to_string(),
                    modify_date: chrono::Utc
                        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                        .single()
                        .expect("Date could not be calculated"),
                    modify_user_name: "User2".to_string(),
                    list_number: 2,
                },
            ])
        }
    }

    #[tokio::test]
    async fn test_get_alarm_lists() {
        let service = AlarmListServiceImpl {
            data_store: Box::new(TestDataStore),
        };
        let result = service.get_alarm_lists(Request::new(EmptyRequest {})).await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert_eq!(response.alarm_lists.len(), 2);
        for (index, value) in response.alarm_lists.iter().enumerate() {
            let index_text = (index + 1).to_string();
            let time = chrono::Utc
                .with_ymd_and_hms(2024, 1, (index + 1).try_into().unwrap(), 0, 0, 0)
                .single()
                .expect("Date could not be calculated");
            assert_eq!(value.name, format!("List{}", index_text));
            assert_eq!(value.long_name, format!("Long List {}", index_text));
            assert_eq!(value.description, format!("Description {}", index_text));
            assert_eq!(
                value.modify_date,
                Some(prost_types::Timestamp {
                    seconds: time.timestamp(),
                    nanos: time.timestamp_subsec_nanos() as i32,
                })
            );
            assert_eq!(value.modify_user_name, format!("User{}", index_text));
            assert_eq!(value.list_number, (index + 1) as i32);
            assert_eq!(
                value.member_devices,
                vec![
                    format!("G:AMANDA{}", index_text),
                    format!("G:AMANDA{}", index_text)
                ]
            ); // Appears twice due to the behavior of TestDataStore
        }
    }
}
