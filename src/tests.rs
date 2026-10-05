//! Main Module Tests

use std::time::Duration;

use rust_db_lib::testing_utils::TestDataStore;
use tokio::time::timeout;

use super::*;

//
// Values for tests with env vars are provided in .cargo/config.toml
//

#[test]
fn test_build_db_config_reads_env_vars() {
    let config = build_db_config().expect("all env vars should be set");

    assert_eq!(config.host, "db.example.com");
    assert_eq!(config.port, 5432u16);
    assert_eq!(config.username, "alice");
    assert_eq!(config.password, "s3cr3t");
    assert_eq!(config.db_name, "alarmsdb");
}

#[test]
fn test_generate_server_address_uses_configured_address_and_port() {
    let addr = generate_server_address().expect("server address should build");

    assert_eq!(addr.ip(), IpAddr::V6(Ipv6Addr::UNSPECIFIED));
    assert_eq!(addr.port(), 7055);
}

#[tokio::test]
async fn test_start_server_wires_all_services() {
    let data_store = TestDataStore::new(vec![]);
    let result = timeout(Duration::from_millis(100), start_server(data_store)).await;

    // The server runs indefinitely — a timeout means it started successfully.
    // An immediate Ok(_) or a panic would indicate a wiring failure.
    result.expect_err("server should still be running after 100ms");
}
