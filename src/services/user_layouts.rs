use std::collections::HashMap;

use tonic::{Request, Response, Status};
use tracing::info;

pub mod proto {
    tonic::include_proto!("services.alarm_user_layouts");
}
use proto::{UserLayout, UserLayouts, user_layouts_service_server::UserLayoutsService};

use crate::db::{DataRow, DataStore, DataStoreError};

/// A service wrapping a DataStore to provide alarm list information, and implementing the Protobuf-defined gRPC service.
pub struct UserLayoutsServiceImpl<T: DataRow> {
    data_store: Box<dyn DataStore<T>>,
}

impl<T: DataRow> UserLayoutsServiceImpl<T> {
    pub fn new(data_store: Box<dyn DataStore<T>>) -> Self {
        Self { data_store }
    }

    /// Retrieves all devices and their associated alarm list.
    ///
    /// Will first attempt to use the device's directly-assigned list. If the list on the device
    /// is 0, will fall back to the list on the device's node.
    async fn get_layouts(&self) -> Result<Vec<UserLayout>, DataStoreError> {
        info!("Query for user layouts ");

        let layout_query = "
            SELECT 
              user_name,
              group_name
            FROM
              alarms.user_layouts
            ORDER BY
              user_name,
              group_name
            ;
            "
        .to_string();

        let rows = self.data_store.execute_query(layout_query).await?;
        let mut sortable_rows = rows
            .into_iter()
            .fold(
                HashMap::new(),
                |mut acc: HashMap<String, UserLayout>, row| {
                    let user_name = row.get_str_value("user_name");
                    let user_layout = acc.entry(user_name.clone()).or_insert(UserLayout {
                        user_name,
                        groups: Vec::new(),
                    });
                    let group_name = row.get_str_value("group_name");
                    user_layout.groups.push(group_name);
                    acc
                },
            )
            .into_values()
            .collect::<Vec<UserLayout>>();
        sortable_rows.sort_by(|a, b| a.user_name.cmp(&b.user_name));
        Ok(sortable_rows)
    }
}

/// Implements the AlarmListService gRPC service.
///
/// Translates query results from the DataStore into gRPC AlarmList messages.
#[tonic::async_trait]
impl<T: DataRow + 'static> UserLayoutsService for UserLayoutsServiceImpl<T> {
    async fn get_user_layouts(&self, _: Request<()>) -> Result<Response<UserLayouts>, Status> {
        let layouts: Vec<UserLayout> = self.get_layouts().await?;
        Ok(Response::new(UserLayouts { layouts }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRow {
        user_name: String,
        group_name: String,
    }
    impl DataRow for TestRow {
        fn get_bool_value(&self, _: &str) -> bool {
            false
        }
        fn get_datetime_value(&self, _: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
        fn get_str_value(&self, column_name: &str) -> String {
            match column_name {
                "user_name" => self.user_name.clone(),
                "group_name" => self.group_name.clone(),
                _ => String::new(),
            }
        }
    }

    struct TestDataStore;
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _: String) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![
                TestRow {
                    group_name: "List1".to_string(),
                    user_name: "User1".to_string(),
                },
                TestRow {
                    group_name: "List2".to_string(),
                    user_name: "User2".to_string(),
                },
            ])
        }
        async fn execute_parameterized_query(
            &self,
            _: String,
            _: Vec<String>,
        ) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_get_user_layouts() {
        let service = UserLayoutsServiceImpl {
            data_store: Box::new(TestDataStore),
        };
        let result = service.get_user_layouts(Request::new(())).await;
        assert!(result.is_ok());
        let response = result.unwrap().into_inner();
        assert_eq!(response.layouts.len(), 2);
        for (index, value) in response.layouts.iter().enumerate() {
            let index_text = (index + 1).to_string();
            assert_eq!(value.user_name, format!("User{}", index_text));
            assert_eq!(value.groups, vec![format!("List{}", index_text)]);
        }
    }
}
