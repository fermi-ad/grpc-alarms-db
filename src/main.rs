use dotenv::dotenv;
use std::{
    env,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::info;

mod db;
mod logging;
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
    start_server(Box::new(data_store)).await
}

async fn start_server<T: DataRow + 'static>(
    data_store: Box<dyn DataStore<T> + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = env::var("ALARM_GRPC_SERVER_PORT")?.parse()?;
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));
    info!("***** Alarm gRPC Server is running at: {} *******", addr);

    let alarm_list_service = AlarmListServiceServer::new(AlarmListServiceImpl { data_store });
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
    }
    struct TestDataStore;
    #[tonic::async_trait]
    impl DataStore<TestRow> for TestDataStore {
        async fn execute_query(&self, _query: &str) -> Vec<TestRow> {
            vec![]
        }
    }

    #[tokio::test]
    async fn test_start_server() {
        dotenv().ok();
        let data_store = Box::new(TestDataStore);
        let future = start_server(data_store);
        let result = timeout(Duration::from_secs(1), future).await;
        assert!(result.is_err());
    }
}
