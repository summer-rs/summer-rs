use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use summer::config::Configurable;

summer::submit_config_schema!("xxl-job", XxlJobConfig);

/// Configuration for the xxl-job / ratch-job executor.
///
/// The TOML section name is `[xxl-job]`. Field names use snake_case in TOML
/// (consistent with other summer plugins).
#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "xxl-job"]
pub struct XxlJobConfig {
    /// Address of xxl-job-admin (or ratch-job admin), e.g.
    /// `http://127.0.0.1:8080/xxl-job-admin`.
    pub admin_addresses: String,

    /// Executor app name registered in admin (`xxl.job.executor.appname`).
    pub app_name: String,

    /// Access token shared between admin and executor. Optional.
    pub access_token: Option<String>,

    /// IP advertised to admin. When omitted the SDK auto-detects the local IP.
    pub ip: Option<String>,

    /// Port of the embedded executor HTTP server. When omitted the SDK picks
    /// a default port.
    pub port: Option<u16>,

    /// Local directory used for xxl-job online log viewing.
    #[serde(default = "default_log_path")]
    pub log_path: String,

    /// Number of days to retain executor logs on disk.
    pub log_retention_days: Option<u32>,

    /// Skip TLS certificate validation when admin uses HTTPS. Dangerous, off
    /// by default.
    #[serde(default)]
    pub ssl_danger_accept_invalid_certs: bool,

    /// Extra HTTP headers attached to every request to admin (e.g. for
    /// gateway authentication).
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_log_path() -> String {
    "logs/xxl-job".to_string()
}
