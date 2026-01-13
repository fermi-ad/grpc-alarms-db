use chrono::Local;

use tracing::Level;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

struct LocalTimer;
impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S"))
    }
}

/// Configures the runtime environment for logging using tracing
pub fn setup_logging() {
    let subscriber = tracing_subscriber::fmt()
        .with_timer(LocalTimer)
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global default subscriber");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "Unable to set global default subscriber")]
    fn test_logging_setup() {
        setup_logging();
        tracing::info!("Logging is set up correctly.");
        setup_logging(); // This should panic
    }
}
