use crate::config::Configurable;
use schemars::JsonSchema;
use serde::Deserialize;
use std::fmt::Display;

impl Configurable for LoggerConfig {
    fn config_prefix() -> &'static str {
        "logger"
    }
}

crate::submit_inventory! {
    crate::config::ConfigSchema {
        prefix: "logger",
        schema: || crate::config::schema_for!(LoggerConfig),
    }
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub(crate) struct LoggerConfig {
    #[serde(default = "default_true")]
    pub enable: bool,

    /// Enable nice display of backtraces, in development this should be on.
    /// Turn it off in performance sensitive production deployments.
    #[serde(default)]
    pub pretty_backtrace: bool,

    /// Set the logger level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    #[serde(default)]
    pub level: LogLevel,

    /// Set the logger format.
    ///
    /// * options: `compact` | `pretty` | `json`
    #[serde(default)]
    pub format: Format,

    /// Formatters for event timestamps.
    #[serde(default)]
    pub time_style: TimeStyle,

    #[serde(default)]
    pub time_pattern: ChronoTimePattern,

    #[serde(default)]
    pub with_fields: Vec<WithFields>,

    /// Override our custom tracing filter.
    ///
    /// Set this to your own filter if you want to see traces from internal
    /// libraries. See more [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives)
    pub override_filter: Option<String>,

    /// Set this if you want to write log to file
    #[serde(rename = "file")]
    pub file_appender: Option<LoggerFileAppender>,
}

#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub(crate) enum LogLevel {
    /// The "off" level.
    #[serde(rename = "off")]
    Off,
    /// The "trace" level.
    #[serde(rename = "trace")]
    Trace,
    /// The "debug" level.
    #[serde(rename = "debug")]
    Debug,
    /// The "info" level.
    #[serde(rename = "info")]
    #[default]
    Info,
    /// The "warn" level.
    #[serde(rename = "warn")]
    Warn,
    /// The "error" level.
    #[serde(rename = "error")]
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Off => "off",
                Self::Trace => "trace",
                Self::Debug => "debug",
                Self::Info => "info",
                Self::Warn => "warn",
                Self::Error => "error",
            }
        )
    }
}

#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub(crate) enum Format {
    #[serde(rename = "compact")]
    #[default]
    Compact,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "json")]
    Json,
}

/// https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/time/index.html
#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub(crate) enum TimeStyle {
    /// Retrieve and print the current wall-clock time.
    #[serde(rename = "system")]
    SystemTime,
    /// Retrieve and print the relative elapsed wall-clock time since an epoch.
    #[serde(rename = "uptime")]
    Uptime,
    /// Formats local times and UTC times with FormatTime implementations that use the chrono crate.
    #[default]
    #[serde(rename = "local")]
    ChronoLocal,
    /// Formats the current UTC time using a formatter from the chrono crate.
    #[serde(rename = "utc")]
    ChronoUtc,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(transparent)]
pub(crate) struct ChronoTimePattern(String);

impl Default for ChronoTimePattern {
    fn default() -> Self {
        Self("%Y-%m-%dT%H:%M:%S".to_string())
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ChronoTimePattern {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WithFields {
    File,
    LineNumber,
    ThreadId,
    ThreadName,
    InternalErrors,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub(crate) struct LoggerFileAppender {
    pub enable: bool,
    #[serde(default = "default_true")]
    pub non_blocking: bool,
    #[serde(default)]
    pub format: Format,
    #[serde(default)]
    pub rotation: Rotation,
    #[serde(default = "default_dir")]
    pub dir: String,
    #[serde(default = "default_prefix")]
    pub filename_prefix: String,
    #[serde(default = "default_suffix")]
    pub filename_suffix: String,
    #[serde(default = "default_max_log_files")]
    pub max_log_files: usize,
}

#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub(crate) enum Rotation {
    #[serde(rename = "minutely")]
    Minutely,
    #[serde(rename = "hourly")]
    Hourly,
    #[serde(rename = "daily")]
    #[default]
    Daily,
    #[serde(rename = "never")]
    Never,
}

fn default_dir() -> String {
    "./logs".to_string()
}

fn default_prefix() -> String {
    "app".to_string()
}

fn default_suffix() -> String {
    "log".to_string()
}

fn default_max_log_files() -> usize {
    365
}

fn default_true() -> bool {
    true
}

#[allow(clippy::from_over_into)]
impl Into<tracing_appender::rolling::Rotation> for Rotation {
    fn into(self) -> tracing_appender::rolling::Rotation {
        match self {
            Self::Minutely => tracing_appender::rolling::Rotation::MINUTELY,
            Self::Hourly => tracing_appender::rolling::Rotation::HOURLY,
            Self::Daily => tracing_appender::rolling::Rotation::DAILY,
            Self::Never => tracing_appender::rolling::Rotation::NEVER,
        }
    }
}
