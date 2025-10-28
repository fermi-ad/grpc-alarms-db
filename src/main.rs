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
use services::alarm_lists::{
    AlarmListServiceImpl, proto, proto::alarm_list_service_server::AlarmListServiceServer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    logging::setup_logging();

    let data_store = PostgresDataStore::new().await;
    start_server(data_store).await
}

async fn start_server<T: DataRow + 'static>(
    data_store: impl DataStore<T> + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = env::var("ALARM_GRPC_SERVER_PORT")?.parse()?;
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    info!("***** Alarm gRPC Server is running at: {} *******", addr);

    let alarm_list_service =
        AlarmListServiceServer::new(AlarmListServiceImpl::new(Box::new(data_store))); // Remember to clone the data store when adding new services
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();
    let result = Server::builder()
        .add_service(alarm_list_service)
        .add_service(reflection_service)
        .serve(addr)
        .await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}

#[cfg(test)]
mod tests {
    use crate::db::DataStoreError;

    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;
    struct TestRow;
    impl DataRow for TestRow {
        fn get_str_value(&self, column_name: &str) -> String {
            column_name.to_string()
        }
        fn get_i32_value(&self, column_name: &str) -> i32 {
            column_name.len() as i32
        }
        fn get_datetime_value(&self, _: &str) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
    }
    struct TestDataStore;
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _query: &str) -> Result<Vec<TestRow>, DataStoreError> {
            Ok(vec![])
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
