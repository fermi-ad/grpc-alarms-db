use tonic::{Request, Response, Status};
use tracing::info;

use proto::{ alarm_list_service_server::AlarmListService, EmptyRequest, AlarmList, AlarmLists };

use crate::db::{DataStore, DataRow};

pub mod proto {
    tonic::include_proto!("alarmlists");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("alarmprotos_descriptor");
}

pub struct AlarmListServiceImpl<T: DataRow> {
    pub data_store: Box<dyn DataStore<T> + Send + Sync>,
}

#[tonic::async_trait]
impl<T: DataRow + 'static> AlarmListService for AlarmListServiceImpl<T> {
    async fn get_alarm_lists(
        &self,
        _request: Request<EmptyRequest>,
    ) -> Result<Response<AlarmLists>, Status> {
        info!("Query for alarm lists ");
        
        let alarm_list_query: &str = "";

        let rows = self.data_store.execute_query(alarm_list_query).await;

        let alarm_lists: Vec<AlarmList> = rows
            .into_iter()
            .map(|row| AlarmList {
                list_number: row.get_i32_value("list_number"),
                name: row.get_str_value("name"),
                long_name: row.get_str_value("long_name"),
                description: row.get_str_value("description"),
                modify_date: row.get_str_value("modify_date"),
                modify_user_name: row.get_str_value("modify_user_name"),
                member_devices: Vec::new(), // Placeholder for member devices
                member_lists: Vec::new(),   // Placeholder for member lists
            })
            .collect();

        let response = AlarmLists { alarm_lists };
        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestRow {
        name: String,
        long_name: String,
        description: String,
        modify_date: String,
        modify_user_name: String,
        list_number: i32,
    }
    impl DataRow for TestRow {
        fn get_str_value(&self, column_name: &str) -> String {
            match column_name {
                "name" => self.name.clone(),
                "long_name" => self.long_name.clone(),
                "description" => self.description.clone(),
                "modify_date" => self.modify_date.clone(),
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
    }
    struct TestDataStore;
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _query: &str) -> Vec<TestRow> {
            vec![
                TestRow {
                    name: "List1".to_string(),
                    long_name: "Long List 1".to_string(),
                    description: "Description 1".to_string(),
                    modify_date: "2024-01-01".to_string(),
                    modify_user_name: "User1".to_string(),
                    list_number: 1,
                },
                TestRow {
                    name: "List2".to_string(),
                    long_name: "Long List 2".to_string(),
                    description: "Description 2".to_string(),
                    modify_date: "2024-01-02".to_string(),
                    modify_user_name: "User2".to_string(),
                    list_number: 2,
                },
            ]
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
            assert_eq!(value.name, format!("List{}", index_text));
            assert_eq!(value.long_name, format!("Long List {}", index_text));
            assert_eq!(value.description, format!("Description {}", index_text));
            assert_eq!(value.modify_date, format!("2024-01-0{}", index_text));
            assert_eq!(value.modify_user_name, format!("User{}", index_text));
            assert_eq!(value.list_number, (index + 1) as i32);
        }
    }
}   
