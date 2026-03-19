//! gRPC Alarms Database service
//!
//! Provides access to persistent data related to alarms via remote procedure calls.
//! Encapsulates the logic for connecting to the database so consuming services are unaware of
//! the specifics of the database implementation.

use rust_db_lib::{DataRow, DataStore, DataVal, postgres::PostgresDataStore};
use rust_env_var_lib::env_var;
use services::{
    alarm_groups::{AlarmGroupServiceServer, AlarmGroupsServiceImpl},
    alarm_timers::{AlarmTimerServiceServer, AlarmTimersServiceImpl},
    user_layouts::{UserLayoutsServiceImpl, UserLayoutsServiceServer},
};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use tonic::transport::Server;
use tracing::info;

mod logging;
mod services;
mod utils;

fn generate_server_address() -> SocketAddr {
    let port = env_var::expect("ALARM_GRPC_SERVER_PORT");
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    info!("***** Alarm gRPC Server is running at: {} *******", addr);
    addr
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
        .add_service(user_layouts_service)
        .serve(generate_server_address())
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
    use rust_db_lib::testing_utils::{TestDataStore, TestVal};
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
