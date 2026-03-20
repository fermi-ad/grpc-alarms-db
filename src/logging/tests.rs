//! Logging Module Tests

use super::*;
use tracing::info;

#[test]
#[should_panic(expected = "Failed to set up logger")]
fn test_logging_setup() {
    setup_logging();
    info!("Logging is set up correctly.");
    setup_logging(); // This should panic
}
