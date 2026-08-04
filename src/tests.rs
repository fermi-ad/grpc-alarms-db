//! Main Module Tests

use super::*;
use rust_db_lib::testing_utils::{TestDataStore, TestVal};
use std::env::set_var;
use std::net::{IpAddr, Ipv6Addr};
use std::time::Duration;
use tokio::time::timeout;

// ── build_db_config ──────────────────────────────────────────────────────────

#[test]
fn test_build_db_config_reads_env_vars() {
    // Safety: test binary is single-process; env mutation is inherently racy
    // across parallel tests. This test owns all five DATABASE_* vars and does
    // not share them with any other test in this module.
    unsafe {
        set_var("DATABASE_HOST", "db.example.com");
        set_var("DATABASE_PORT", "5432");
        set_var("DATABASE_USER", "alice");
        set_var("DATABASE_PASS", "s3cr3t");
        set_var("DATABASE_NAME", "alarmsdb");
    }

    let config = build_db_config();

    assert_eq!(config.host, "db.example.com");
    assert_eq!(config.port, 5432u16);
    assert_eq!(config.username, "alice");
    assert_eq!(config.password, "s3cr3t");
    assert_eq!(config.db_name, "alarmsdb");
}

// ── generate_server_address ──────────────────────────────────────────────────

// ALARM_GRPC_SERVER_PORT = "7055" is injected for all tests via .cargo/config.toml.
// This test is read-only with respect to env vars and is safe to run in parallel.

#[test]
fn test_generate_server_address_uses_configured_address_and_port() {
    let addr = generate_server_address();

    assert_eq!(addr.ip(), IpAddr::V6(Ipv6Addr::UNSPECIFIED));
    assert_eq!(addr.port(), 7055);
}

// ── Integration ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct TestRow;
impl DataRow<TestVal> for TestRow {
    fn get(&self, _: &str) -> TestVal {
        TestVal::new()
    }
}

#[tokio::test]
async fn test_start_server_wires_all_services() {
    let data_store = TestDataStore::new(Vec::<TestRow>::new());
    let result = timeout(Duration::from_millis(100), start_server(data_store)).await;

    // The server runs indefinitely — a timeout means it started successfully.
    // An immediate Ok(_) or a panic would indicate a wiring failure.
    assert!(
        result.is_err(),
        "server should still be running after 100ms"
    );
}
