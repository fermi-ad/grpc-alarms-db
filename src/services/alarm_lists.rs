use std::collections::HashMap;

use tonic::{Request, Response, Status};
use tracing::info;

use proto::{AlarmList, AlarmLists, alarm_list_service_server::AlarmListService};

use crate::db::{DataRow, DataStore, DataStoreError};

pub mod proto {
    tonic::include_proto!("services.alarm_lists");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("alarmprotos_descriptor");
}

/// A service wrapping a DataStore to provide alarm list information, and implementing the Protobuf-defined gRPC service.
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

    /// Retrieves all devices and their associated alarm list.
    ///
    /// Will first attempt to use the device's directly-assigned list. If the list on the device
    /// is 0, will fall back to the list on the device's node.
    async fn get_devices_and_lists(&self) -> Result<Vec<AlarmList>, DataStoreError> {
        info!("Query for alarm lists and associated devices ");

        let alarm_list_query: &str = "
            SELECT DISTINCT
              l.list_number, 
              l.name AS list_name,
              l.long_name, 
              l.description, 
              l.modify_date, 
              l.modify_user_name, 
              d.name AS device_name
            FROM 
              accdb.alarm_block b
              INNER JOIN
                accdb.device d
                ON
                  d.di = b.di
              INNER JOIN 
                hendricks.alarm_list_nodes n 
                ON 
                  d.trunk = n.trunk 
                AND 
                  d.node = n.node
              INNER JOIN
                hendricks.alarm_list_info l
                ON
                  CASE
                    WHEN d.alarm_list_id > 0 THEN d.alarm_list_id = l.list_number
                    ELSE n.list_number = l.list_number
                  END             
            ORDER BY 
              l.list_number, 
              d.name
            ;
            ";

        let rows = self.data_store.execute_query(alarm_list_query).await?;
        let mut sortable_rows = rows
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
            })
            .into_values()
            .collect::<Vec<AlarmList>>();
        sortable_rows.sort_by(|a, b| a.list_number.cmp(&b.list_number));
        Ok(sortable_rows)
    }
}

/// Implements the AlarmListService gRPC service.
///
/// Translates query results from the DataStore into gRPC AlarmList messages.
#[tonic::async_trait]
impl<T: DataRow + 'static> AlarmListService for AlarmListServiceImpl<T> {
    async fn get_alarm_lists(&self, _: Request<()>) -> Result<Response<AlarmLists>, Status> {
        let alarm_lists: Vec<AlarmList> = self.get_devices_and_lists().await?;
        Ok(Response::new(AlarmLists { alarm_lists }))
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
        let result = service.get_alarm_lists(Request::new(())).await;
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
                vec![format!("G:AMANDA{}", index_text)]
            );
        }
    }
}
