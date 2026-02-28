use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;

summer::submit_config_schema!("sqlx", SqlxConfig);

#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "sqlx"]
#[allow(clippy::struct_excessive_bools)]
pub struct SqlxConfig {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,

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
