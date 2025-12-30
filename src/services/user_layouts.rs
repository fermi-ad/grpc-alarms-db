use std::{collections::HashMap, marker::PhantomData};

use tonic::{Request, Response, Status};
use tracing::{error, info};

pub mod proto {
    tonic::include_proto!("services.alarm_user_layouts");
}
use proto::{UserLayout, UserLayouts, user_layouts_service_server::UserLayoutsService};

use rust_db_lib::{DataRow, DataStore, DataStoreError, DataVal};

/// A service wrapping a DataStore to provide alarm list information, and implementing the Protobuf-defined gRPC service.
pub struct UserLayoutsServiceImpl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> {
    data_store: V,
    _row_type: PhantomData<U>,
    _val_type: PhantomData<T>,
}

impl<T: DataVal + 'static, U: DataRow<T> + 'static, V: DataStore<T, U> + 'static>
    UserLayoutsServiceImpl<T, U, V>
{
    pub fn new(data_store: V) -> Self {
        Self {
            data_store,
            _row_type: PhantomData,
            _val_type: PhantomData,
        }
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
              alarmsapp.user_layouts
            ORDER BY
              user_name,
              group_name
            ;
            "
        .to_string();

        let rows = self.data_store.execute_query(layout_query).await?;
        let mut layout_builder = HashMap::new();
        for row in rows {
            let user_name = row.get("user_name").to_string()?;
            let group_name = row.get("group_name").to_string()?;
            layout_builder
                .entry(user_name.clone())
                .or_insert_with(|| UserLayout {
                    user_name,
                    groups: Vec::new(),
                })
                .groups
                .push(group_name);
        }
        let mut sortable_rows = layout_builder.into_values().collect::<Vec<UserLayout>>();
        sortable_rows.sort_by(|a, b| a.user_name.cmp(&b.user_name));
        Ok(sortable_rows)
    }
}

/// Implements the AlarmListService gRPC service.
///
/// Translates query results from the DataStore into gRPC AlarmList messages.
#[tonic::async_trait]
impl<T: DataVal + 'static, U: DataRow<T> + 'static, V: DataStore<T, U> + 'static> UserLayoutsService
    for UserLayoutsServiceImpl<T, U, V>
{
    async fn get_user_layouts(&self, _: Request<()>) -> Result<Response<UserLayouts>, Status> {
        match self.get_layouts().await {
            Ok(layouts) => Ok(Response::new(UserLayouts { layouts })),
            Err(e) => {
                error!("{}", e);
                Err(Status::internal(
                    "Failed to retrieve user layouts. See server logs for details.",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_db_lib::test_utils::TestVal;

    struct TestRow {
        user_name: String,
        group_name: String,
    }
    impl DataRow<TestVal> for TestRow {
        fn get(&self, column_name: &str) -> TestVal {
            match column_name {
                "user_name" => {
                    let mut result = TestVal::new();
                    result.test_string = Some(self.user_name.clone());
                    result
                }
                "group_name" => {
                    let mut result = TestVal::new();
                    result.test_string = Some(self.group_name.clone());
                    result
                }
                _ => TestVal::new(),
            }
        }
    }

    struct TestDataStore;
    impl Clone for TestDataStore {
        fn clone(&self) -> Self {
            Self
        }
    }
    #[tonic::async_trait]
    impl DataStore<TestVal, TestRow> for TestDataStore {
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
            data_store: TestDataStore {},
            _row_type: PhantomData,
            _val_type: PhantomData,
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
