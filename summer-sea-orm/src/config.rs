use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;

summer::submit_config_schema!("sea-orm", SeaOrmConfig);
#[cfg(feature = "with-web")]
summer::submit_config_schema!("sea-orm-web", SeaOrmWebConfig);

#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "sea-orm"]
pub struct SeaOrmConfig {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,

    /// Enable `SQLx` statement logging
    #[serde(default)]
    pub enable_logging: bool,

    /// Minimum number of connections for a pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Maximum number of connections for a pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Set the timeout duration when acquiring a connection
    pub connect_timeout: Option<u64>,

    /// Set a maximum idle duration for individual connections.
    /// Any connection that remains in the idle queue longer than this will be closed.
    /// For usage-based database server billing, this can be a cost saver.
    pub idle_timeout: Option<u64>,

    /// Set the timeout for acquiring a connection
    pub acquire_timeout: Option<u64>,
}

fn default_min_connections() -> u32 {
    1
}

fn default_max_connections() -> u32 {
    10
}

#[cfg(feature = "with-web")]
#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "sea-orm-web"]
pub struct SeaOrmWebConfig {
    /// Configures whether to expose and assume 1-based page number indexes in the request parameters.
    /// Defaults to false, meaning a page number of 0 in the request equals the first page.
    /// If this is set to true, a page number of 1 in the request will be considered the first page.
    #[serde(default = "default_one_indexed")]
    pub one_indexed: bool,

    /// Configures the maximum page size to be accepted.
    /// This allows to put an upper boundary of the page size to prevent potential attacks trying to issue an OOM.
    /// Defaults to 2000.
    #[serde(default = "default_max_page_size")]
    pub max_page_size: u64,

    /// Default page size.
    #[serde(default = "default_page_size")]
    pub default_page_size: u64,
}

#[allow(dead_code)]
fn default_one_indexed() -> bool {
    false
}

#[allow(dead_code)]
fn default_max_page_size() -> u64 {
    2000
}

#[allow(dead_code)]
fn default_page_size() -> u64 {
    20
}
