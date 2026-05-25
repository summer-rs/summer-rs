#![doc = include_str!("../../Log-Plugin.md")]
mod bootstrap;
mod config;

pub use bootstrap::install_bootstrap_logger;
pub use bootstrap::BoxLayer;
pub(crate) use config::{LogLevel, LoggerConfig};

use crate::app::AppBuilder;
use crate::config::ConfigRegistry;
use crate::plugin::Plugin;
use config::{Format, TimeStyle, WithFields};
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::time::{ChronoLocal, ChronoUtc, FormatTime, SystemTime, Uptime};
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::{self, MakeWriter};

/// Built-in Log plugin based on [tracing](https://docs.rs/tracing)
pub(crate) struct LogPlugin;

impl Plugin for LogPlugin {
    fn immediately_build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<LoggerConfig>()
            .expect("tracing plugin config load failed");

        if config.pretty_backtrace {
            std::env::set_var("RUST_BACKTRACE", "1");
            log::warn!(
                "pretty backtraces are enabled (this is great for development but has a runtime cost for production. disable with `logger.pretty_backtrace` in your config)"
            );
        }

        // Compose the user-configured layer stack and push `ErrorLayer`
        // onto the end so backtrace context is captured.
        let layers = std::mem::take(&mut app.layers);
        let mut user_layers = config.config_subscriber(layers);
        user_layers.push(Box::new(ErrorLayer::default()));
        let env_filter = config.build_env_filter();

        // Hot-swap both reload slots that `install_bootstrap_logger`
        // wired up. The global dispatcher is already in place, so we
        // never call `set_global_default` again.
        let mut hot_swapped = false;
        if let Some(handle) = bootstrap::LAYER_RELOAD_HANDLE.get() {
            match handle.modify(|slot| *slot = user_layers) {
                Ok(()) => hot_swapped = true,
                Err(e) => eprintln!("LogPlugin: layer reload failed: {e:?}"),
            }
        }
        if let Some(handle) = bootstrap::FILTER_RELOAD_HANDLE.get() {
            if let Err(e) = handle.modify(|slot| *slot = env_filter) {
                eprintln!("LogPlugin: filter reload failed: {e:?}");
            }
        }

        if hot_swapped {
            // Callsites encountered during the bootstrap window cached
            // their `Interest` against the placeholder filter; rebuild
            // so the new user filter applies retroactively.
            tracing::callsite::rebuild_interest_cache();
        }
    }

    fn immediately(&self) -> bool {
        true
    }
}

// Keep nonblocking file appender work guard
static NONBLOCKING_WORK_GUARD_KEEP: OnceLock<WorkerGuard> = OnceLock::new();

impl LoggerConfig {
    fn config_subscriber(&self, mut layers: Vec<BoxLayer>) -> Vec<BoxLayer> {
        if let Some(file_logger) = &self.file_appender {
            if file_logger.enable {
                let file_appender = tracing_appender::rolling::Builder::default()
                    .max_log_files(file_logger.max_log_files)
                    .filename_prefix(&file_logger.filename_prefix)
                    .filename_suffix(&file_logger.filename_suffix)
                    .rotation(file_logger.rotation.clone().into())
                    .build(&file_logger.dir)
                    .expect("logger file appender initialization failed");

                let file_appender_layer = if file_logger.non_blocking {
                    let (non_blocking_file_appender, work_guard) =
                        tracing_appender::non_blocking(file_appender);
                    NONBLOCKING_WORK_GUARD_KEEP.set(work_guard).unwrap();
                    self.build_fmt_layer(non_blocking_file_appender, &file_logger.format, false)
                } else {
                    self.build_fmt_layer(file_appender, &file_logger.format, false)
                };
                layers.push(file_appender_layer);
            }
        }

        if self.enable {
            layers.push(self.build_fmt_layer(std::io::stdout, &self.format, true));
        }

        layers
    }

    fn build_fmt_layer<W2>(&self, make_writer: W2, format: &Format, ansi: bool) -> BoxLayer
    where
        W2: for<'writer> MakeWriter<'writer> + Sync + Send + 'static,
    {
        let Self {
            time_style,
            time_pattern,
            ..
        } = &self;
        match time_style {
            TimeStyle::SystemTime => {
                self.build_layer_with_timer(make_writer, format, SystemTime, ansi)
            }
            TimeStyle::Uptime => {
                self.build_layer_with_timer(make_writer, format, Uptime::default(), ansi)
            }
            TimeStyle::ChronoLocal => self.build_layer_with_timer(
                make_writer,
                format,
                ChronoLocal::new(time_pattern.to_string()),
                ansi,
            ),
            TimeStyle::ChronoUtc => self.build_layer_with_timer(
                make_writer,
                format,
                ChronoUtc::new(time_pattern.to_string()),
                ansi,
            ),
            TimeStyle::None => self.build_layer_without_timer(make_writer, format, ansi),
        }
    }

    fn build_layer_with_timer<W2, T>(
        &self,
        make_writer: W2,
        format: &Format,
        timer: T,
        ansi: bool,
    ) -> BoxLayer
    where
        W2: for<'writer> MakeWriter<'writer> + Sync + Send + 'static,
        T: FormatTime + Sync + Send + 'static,
    {
        let mut layer = fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .with_timer(timer);

        for field in &self.with_fields {
            match field {
                WithFields::File => layer = layer.with_file(true),
                WithFields::LineNumber => layer = layer.with_line_number(true),
                WithFields::ThreadId => layer = layer.with_thread_ids(true),
                WithFields::ThreadName => layer = layer.with_thread_names(true),
                WithFields::InternalErrors => layer = layer.log_internal_errors(true),
            }
        }

        match format {
            Format::Compact => layer.compact().boxed(),
            Format::Pretty => layer.pretty().boxed(),
            Format::Json => layer.json().boxed(),
        }
    }

    fn build_layer_without_timer<W2>(
        &self,
        make_writer: W2,
        format: &Format,
        ansi: bool,
    ) -> BoxLayer
    where
        W2: for<'writer> MakeWriter<'writer> + Sync + Send + 'static,
    {
        let mut layer = fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .without_time();

        for field in &self.with_fields {
            match field {
                WithFields::File => layer = layer.with_file(true),
                WithFields::LineNumber => layer = layer.with_line_number(true),
                WithFields::ThreadId => layer = layer.with_thread_ids(true),
                WithFields::ThreadName => layer = layer.with_thread_names(true),
                WithFields::InternalErrors => layer = layer.log_internal_errors(true),
            }
        }

        match format {
            Format::Compact => layer.compact().boxed(),
            Format::Pretty => layer.pretty().boxed(),
            Format::Json => layer.json().boxed(),
        }
    }

    fn build_env_filter(&self) -> EnvFilter {
        match EnvFilter::try_from_default_env() {
            Ok(env_filter) => env_filter,
            Err(_) => {
                let LoggerConfig {
                    override_filter,
                    level,
                    ..
                } = self;
                let directive = match override_filter {
                    Some(dir) => dir.into(),
                    None => format!("{level}"),
                };
                EnvFilter::try_new(directive).expect("logger initialization failed")
            }
        }
    }
}
