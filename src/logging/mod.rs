//! Logging Module
//!
//! Configures logging for the application.

use tracing_subscriber::{Registry, filter::EnvFilter, fmt::layer, layer::SubscriberExt};

#[cfg(test)]
mod tests;

/// Configures the runtime environment for logging using tracing
pub fn setup_logging() {
    let fmt_layer = layer()
        .with_target(false)
        .with_file(true)
        .with_line_number(true);
    // The following reads the log levels specified in the RUST_LOG environment variable. Allows us to configure logging
    // at both the application level and for specific crates/modules.
    let level_layer = EnvFilter::from_default_env();
    let subscriber = Registry::default().with(fmt_layer).with(level_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set up logger");
}
