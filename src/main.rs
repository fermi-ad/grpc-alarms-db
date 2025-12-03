use dotenv::dotenv;
use std::{
    env,
    net::{IpAddr, Ipv6Addr, SocketAddr},
};
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::info;

mod logging;

mod db;
use db::{DataRow, DataStore, postgres::PostgresDataStore};

mod services;
use services::alarm_groups::{
    AlarmGroupsServiceImpl, proto, proto::alarm_group_service_server::AlarmGroupServiceServer,
};
use services::user_layouts::{
    UserLayoutsServiceImpl, proto::user_layouts_service_server::UserLayoutsServiceServer,
};

fn generate_server_address() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let port: u16 = env::var("ALARM_GRPC_SERVER_PORT")?.parse()?;
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    info!("***** Alarm gRPC Server is running at: {} *******", addr);
    Ok(addr)
}

async fn start_server<T: DataRow + 'static, U: DataStore<T> + 'static>(
    data_store: U,
) -> Result<(), Box<dyn std::error::Error>> {
    let user_layouts_service =
        UserLayoutsServiceServer::new(UserLayoutsServiceImpl::new(Box::new(data_store.clone()))); // Remember to clone the data store when adding new services
    let alarm_group_service =
        AlarmGroupServiceServer::new(AlarmGroupsServiceImpl::new(Box::new(data_store)));
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<UserLayoutsServiceServer<U>>()
        .await;
    health_reporter
        .set_serving::<AlarmGroupServiceServer<U>>()
        .await;
    let result = Server::builder()
        .add_service(user_layouts_service)
        .add_service(alarm_group_service)
        .add_service(reflection_service)
        .add_service(health_service)
        .serve(generate_server_address().unwrap())
        .await;
    health_reporter
        .set_not_serving::<UserLayoutsServiceServer<U>>()
        .await;
    health_reporter
        .set_not_serving::<AlarmGroupServiceServer<U>>()
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    logging::setup_logging();

    let data_store = PostgresDataStore::new().await;
    start_server(data_store).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DataStoreError;
    use std::time::Duration;
    use tokio::time::timeout;

    struct TestRow;
    impl DataRow for TestRow {
        fn get_bool_value(&self, _: &str) -> bool {
            false
        }
        fn get_datetime_value(&self, _: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
        fn get_str_value(&self, column_name: &str) -> String {
            column_name.to_string()
        }
    }
    struct TestDataStore;
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _: String) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![])
        }
        async fn execute_parameterized_query(
            &self,
            _: String,
            _: Vec<String>,
        ) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![])
        }
    }
    impl Clone for TestDataStore {
        fn clone(&self) -> Self {
            TestDataStore {}
        }
    }

    #[tokio::test]
    async fn test_start_server() {
        dotenv().ok();
        let data_store = TestDataStore {};
        let future = start_server(data_store);
        let result = timeout(Duration::from_secs(1), future).await;
        assert!(result.is_err());
    }
}
