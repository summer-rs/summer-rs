use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use summer::config::Configurable;

summer::submit_config_schema!("nacos", NacosConfig);

fn default_true() -> bool {
    true
}

/// Nacos / [r-nacos](https://github.com/nacos-group/r-nacos) client configuration.
#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "nacos"]
pub struct NacosConfig {
    /// Nacos server address, e.g. `127.0.0.1:8848`.
    #[serde(default = "default_server_addr")]
    pub server_addr: String,
    /// Namespace id. Use empty string for the public namespace.
    #[serde(default)]
    pub namespace: String,
    /// Application name reported to Nacos.
    pub app_name: String,
    /// Enable HTTP auth plugin (required when r-nacos `RNACOS_ENABLE_OPEN_API_AUTH=true`).
    #[serde(default)]
    pub enable_auth: bool,
    /// Auth username.
    pub username: Option<String>,
    /// Auth password.
    pub password: Option<String>,
    /// Create a config center client.
    #[serde(default = "default_true")]
    pub enable_config: bool,
    /// Create a naming (service discovery) client.
    #[serde(default)]
    pub enable_naming: bool,
    /// Remote TOML configs to pull from Nacos on [`ConfigEvent`] and merge into the registry
    /// (later entries override earlier ones).
    ///
    /// TOML: use `[[nacos.bootstrap]]` for multiple entries, or `[nacos.bootstrap]` for one.
    #[serde(default, deserialize_with = "deserialize_bootstrap_list")]
    pub bootstrap: Vec<NacosBootstrapConfig>,
    /// Register this instance to Nacos when a server starts ([`summer::event::ServerStartedEvent`]).
    #[serde(default)]
    pub registration: Option<NacosRegistrationConfig>,
}

fn default_server_addr() -> String {
    "127.0.0.1:8848".to_string()
}

/// One-shot config fetch at plugin initialization.
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct NacosBootstrapConfig {
    pub data_id: String,
    #[serde(default = "default_group")]
    pub group: String,
}

fn default_group() -> String {
    "DEFAULT_GROUP".to_string()
}

/// 兼容 TOML 两种写法：`[nacos.bootstrap]`（单个）与 `[[nacos.bootstrap]]`（多个）。
fn deserialize_bootstrap_list<'de, D>(deserializer: D) -> Result<Vec<NacosBootstrapConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        One(NacosBootstrapConfig),
        Many(Vec<NacosBootstrapConfig>),
    }

    match Repr::deserialize(deserializer)? {
        Repr::One(one) => Ok(vec![one]),
        Repr::Many(many) => Ok(many),
    }
}

/// Service instance registration settings.
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct NacosRegistrationConfig {
    pub service_name: String,
    #[serde(default = "default_group")]
    pub group: String,
    /// Instance IP. When omitted, uses the address from [`summer::event::ServerStartedEvent`] or local IP.
    pub ip: Option<String>,
    /// Instance port. When omitted, uses the port from each [`summer::event::ServerStartedEvent`]
    /// (recommended when both HTTP and gRPC register under the same service name).
    pub port: Option<u16>,
    #[serde(default)]
    pub weight: f64,
    pub cluster: Option<String>,
    /// Extra instance metadata. A `protocol` key here overrides the value from [`ServerStartedEvent`].
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Default for NacosRegistrationConfig {
    fn default() -> Self {
        Self {
            service_name: String::new(),
            group: default_group(),
            ip: None,
            port: None,
            weight: 1.0,
            cluster: None,
            metadata: HashMap::new(),
        }
    }
}
