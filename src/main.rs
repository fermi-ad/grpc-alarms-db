mod logging;

mod proto {
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("alarmprotos_descriptor");
}

use rust_db_lib::{DataRow, DataStore, DataVal, postgres::PostgresDataStore};
use rust_env_var_lib::env_var;

mod services;
use services::alarm_groups::{AlarmGroupServiceServer, AlarmGroupsServiceImpl};
use services::alarm_timers::{AlarmTimerServiceServer, AlarmTimersServiceImpl};
use services::user_layouts::{UserLayoutsServiceImpl, UserLayoutsServiceServer};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::info;

mod utils;

fn generate_server_address() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let port = env_var::get("ALARM_GRPC_SERVER_PORT").or(7055_u16);
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    info!("***** Alarm gRPC Server is running at: {} *******", addr);
    Ok(addr)
}

async fn start_server<
    T: DataVal + 'static,
    U: DataRow<T> + 'static,
    V: DataStore<T, U> + 'static,
>(
    data_store: V,
) -> Result<(), Box<dyn std::error::Error>> {
    let alarm_group_service =
        AlarmGroupServiceServer::new(AlarmGroupsServiceImpl::new(data_store.clone()));
    let alarm_timer_service =
        AlarmTimerServiceServer::new(AlarmTimersServiceImpl::new(data_store.clone()));
    let user_layouts_service =
        UserLayoutsServiceServer::new(UserLayoutsServiceImpl::new(data_store));
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AlarmGroupServiceServer<V>>()
        .await;
    health_reporter
        .set_serving::<AlarmTimerServiceServer<V>>()
        .await;
    health_reporter
        .set_serving::<UserLayoutsServiceServer<V>>()
        .await;
    let result = Server::builder()
        .add_service(alarm_group_service)
        .add_service(alarm_timer_service)
        .add_service(health_service)
        .add_service(reflection_service)
        .add_service(user_layouts_service)
        .serve(generate_server_address().unwrap())
        .await;
    health_reporter
        .set_not_serving::<AlarmGroupServiceServer<V>>()
        .await;
    health_reporter
        .set_not_serving::<AlarmTimerServiceServer<V>>()
        .await;
    health_reporter
        .set_not_serving::<UserLayoutsServiceServer<V>>()
        .await;
    Ok(result?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging();

    let data_store = PostgresDataStore::new().await;
    start_server(data_store).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_db_lib::test_utils::{TestDataStore, TestVal};
    use std::time::Duration;
    use tokio::time::timeout;

    #[derive(Clone, Debug)]
    struct TestRow;
    impl DataRow<TestVal> for TestRow {
        fn get(&self, _: &str) -> TestVal {
            TestVal::new()
        }
    }

    #[tokio::test]
    async fn test_start_server() {
        let data_store = TestDataStore::new(Vec::<TestRow>::new());
        let future = start_server(data_store);
        let result = timeout(Duration::from_secs(1), future).await;
        assert!(result.is_err());
    }
}
