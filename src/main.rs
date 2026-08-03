//! gRPC Alarms Database service
//!
//! Provides access to persistent data related to alarms via remote procedure calls.
//! Encapsulates the logic for connecting to the database so consuming services are unaware of
//! the specifics of the database implementation.

use proto::services::{
    alarm_groups::alarm_group_service_server::AlarmGroupServiceServer,
    alarm_timers::alarm_timer_service_server::AlarmTimerServiceServer,
    alarm_user_layouts::user_layouts_service_server::UserLayoutsServiceServer,
};
use rust_db_lib::{
    DataRow, DataStore, DataVal,
    postgres::{PostgresConfig, PostgresDataStore},
};
use rust_env_var_lib::env_var;
use services::{
    alarm_groups::AlarmGroupsServiceImpl, alarm_timers::AlarmTimersServiceImpl,
    user_layouts::UserLayoutsServiceImpl,
};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use tonic::transport::Server;
use tracing::info;

mod logging;
mod proto;
mod services;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::setup_logging();

    let config = build_db_config();
    let data_store = PostgresDataStore::new(config).await?;
    start_server(data_store).await
}

fn build_db_config() -> PostgresConfig {
    PostgresConfig {
        host: env_var::expect("DATABASE_HOST"),
        port: env_var::expect("DATABASE_PORT"),
        username: env_var::expect("DATABASE_USER"),
        password: env_var::expect("DATABASE_PASS"),
        db_name: env_var::expect("DATABASE_NAME"),
        ..Default::default()
    }
}

fn generate_server_address() -> SocketAddr {
    let port = env_var::expect("ALARM_GRPC_SERVER_PORT");
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
    info!("***** Alarm gRPC Server is running at: {addr} *******");
    addr
}

async fn start_server<T: DataVal, U: DataRow<T>, V: DataStore<T, U>>(
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
