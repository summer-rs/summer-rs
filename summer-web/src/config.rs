use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;
use std::net::{IpAddr, Ipv4Addr};
use tracing::Level;

summer::submit_config_schema!("web", WebConfig);

#[cfg(feature = "socket_io")]
summer::submit_config_schema!("socket_io", SocketIOConfig);

/// summer-web Config
#[derive(Debug, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "web"]
pub struct WebConfig {
    #[serde(flatten)]
    pub(crate) server: ServerConfig,
    #[cfg(feature = "openapi")]
    pub(crate) openapi: OpenApiConfig,
    pub(crate) middlewares: Option<Middlewares>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_binding")]
    pub(crate) binding: IpAddr,
    #[serde(default = "default_port")]
    pub(crate) port: u16,
    #[serde(default)]
    pub(crate) connect_info: bool,
    #[serde(default = "default_true")]
    pub(crate) graceful: bool,
    #[serde(default)]
    pub(crate) global_prefix: String,
}

#[cfg(feature = "openapi")]
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct OpenApiConfig {
    #[serde(default = "default_doc_prefix")]
    pub(crate) doc_prefix: String,
    #[serde(default)]
    pub(crate) info: aide::openapi::Info,
}

fn default_binding() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

fn default_port() -> u16 {
    8080
}

fn default_true() -> bool {
    true
}

#[cfg(feature = "openapi")]
fn default_doc_prefix() -> String {
    "/docs".into()
}

/// Server middleware configuration structure.
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct Middlewares {
    /// Middleware that enable compression for the response.
    pub compression: Option<EnableMiddleware>,
    /// Middleware that limit the payload request.
    pub limit_payload: Option<LimitPayloadMiddleware>,
    /// Middleware that improve the tracing logger and adding trace id for each
    /// request.
    pub logger: Option<TraceLoggerMiddleware>,
    /// catch any code panic and log the error.
    pub catch_panic: Option<EnableMiddleware>,
    /// Setting a global timeout for the requests
    pub timeout_request: Option<TimeoutRequestMiddleware>,
    /// Setting cors configuration
    pub cors: Option<CorsMiddleware>,
    /// Serving static assets
    #[serde(rename = "static")]
    pub static_assets: Option<StaticAssetsMiddleware>,
}

/// Static asset middleware configuration
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct StaticAssetsMiddleware {
    /// toggle enable
    pub enable: bool,
    /// Check that assets must exist on disk
    #[serde(default = "bool::default")]
    pub must_exist: bool,
    /// Fallback page for a case when no asset exists (404). Useful for SPA
    /// (single page app) where routes are virtual.
    #[serde(default = "default_fallback")]
    pub fallback: String,
    /// Enable `precompressed_gzip`
    #[serde(default = "bool::default")]
    pub precompressed: bool,
    /// Uri for the assets
    #[serde(default = "default_assets_uri")]
    pub uri: String,
    /// Path for the assets
    #[serde(default = "default_assets_path")]
    pub path: String,
}

/// CORS middleware configuration
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct TraceLoggerMiddleware {
    /// toggle enable
    pub enable: bool,
    pub level: LogLevel,
}

#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub enum LogLevel {
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

#[allow(clippy::from_over_into)]
impl Into<Level> for LogLevel {
    fn into(self) -> Level {
        match self {
            Self::Trace => Level::TRACE,
            Self::Debug => Level::DEBUG,
            Self::Info => Level::INFO,
            Self::Warn => Level::WARN,
            Self::Error => Level::ERROR,
        }
    }
}

/// CORS middleware configuration
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct CorsMiddleware {
    /// toggle enable
    pub enable: bool,
    /// Allow origins
    pub allow_origins: Option<Vec<String>>,
    /// Allow headers
    pub allow_headers: Option<Vec<String>>,
    /// Allow methods
    pub allow_methods: Option<Vec<String>>,
    /// Max age
    pub max_age: Option<u64>,
}

/// Timeout middleware configuration
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct TimeoutRequestMiddleware {
    /// toggle enable
    pub enable: bool,
    /// Timeout request in milliseconds
    pub timeout: u64,
}

/// Limit payload size middleware configuration
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct LimitPayloadMiddleware {
    /// toggle enable
    pub enable: bool,
    /// Body limit. for example: 5mb
    pub body_limit: String,
}

/// A generic middleware configuration that can be enabled or
/// disabled.
#[derive(Debug, PartialEq, Clone, JsonSchema, Deserialize)]
pub struct EnableMiddleware {
    /// toggle enable
    pub enable: bool,
}

fn default_assets_path() -> String {
    "static".to_string()
}

fn default_assets_uri() -> String {
    "/static".to_string()
}

fn default_fallback() -> String {
    "index.html".to_string()
}

/// SocketIO configuration
#[cfg(feature = "socket_io")]
#[derive(Debug, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "socket_io"]
pub struct SocketIOConfig {
    #[serde(default = "default_namespace")]
    pub default_namespace: String,
}

#[cfg(feature = "socket_io")]
fn default_namespace() -> String {
    "/".to_string()
}
