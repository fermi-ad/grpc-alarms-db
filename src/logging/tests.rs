//! Logging Module Tests

use super::*;

#[test]
fn test_logging_setup() {
    setup_logging().expect("Initial log setup should succeed");
    let _ = setup_logging().expect_err("Duplicate log initialization should fail");
}
