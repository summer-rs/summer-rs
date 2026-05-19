//! Stderr [`log`] backend used before [`super::LogPlugin`] installs tracing.

use std::sync::Once;

static INSTALL: Once = Once::new();
static LOGGER: BootstrapLogger = BootstrapLogger;

struct BootstrapLogger;

impl log::Log for BootstrapLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "{} {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn bootstrap_max_level() -> log::LevelFilter {
    match std::env::var("RUST_LOG")
        .ok()
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("trace") => log::LevelFilter::Trace,
        Some("debug") => log::LevelFilter::Debug,
        Some("warn") => log::LevelFilter::Warn,
        Some("error") => log::LevelFilter::Error,
        Some("off") => log::LevelFilter::Off,
        _ => log::LevelFilter::Info,
    }
}

/// Installs a stderr [`log`] logger when no global tracing subscriber exists yet.
///
/// Until [`super::LogPlugin`] runs, `tracing` events (with the `log` feature) and
/// `log::` records are written to stderr. After LogPlugin initializes tracing,
/// [`super::wire_log_to_tracing`] replaces this logger with [`tracing_log::LogTracer`].
pub fn install_bootstrap_logger() {
    INSTALL.call_once(|| {
        if tracing::dispatcher::has_been_set() {
            return;
        }
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(bootstrap_max_level());
        }
    });
}
