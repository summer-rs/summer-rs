//! This module implements configuration loading.
//!
#![doc = include_str!("../../Config.md")]
/// Environment Configuration
pub mod env;
/// Implement reading toml configuration
pub mod toml;

pub use schemars::schema_for;
pub use schemars::Schema;
pub use summer_macros::Configurable;

use crate::error::Result;
use serde_json::json;
use std::{ops::Deref, sync::Arc};

/// The Configurable trait marks whether the struct can read configuration from the [ConfigRegistry]
pub trait Configurable {
    /// Prefix used to read toml configuration.
    /// If you need to load external configuration, you need to rewrite this method
    fn config_prefix() -> &'static str;
}

/// ConfigRegistry is the core trait of configuration management
pub trait ConfigRegistry {
    /// Get the configuration items according to the Configurable's `config_prefix`
    fn get_config<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Configurable;
}

/// ConfigRef avoids cloning of big struct through Arc
#[derive(Debug, Clone)]
pub struct ConfigRef<T: Configurable>(Arc<T>);

impl<T: Configurable> ConfigRef<T> {
    /// constructor
    pub fn new(config: T) -> Self {
        Self(Arc::new(config))
    }
}

impl<T> Deref for ConfigRef<T>
where
    T: Configurable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Collects all configured schema generation information
pub struct ConfigSchema {
    /// [`Configurable::config_prefix`]
    pub prefix: &'static str,
    /// Generate the [`Schema`] for this configuration
    pub schema: fn() -> Schema,
}

inventory::collect!(ConfigSchema);

/// register config schema
#[macro_export]
macro_rules! submit_config_schema {
    ($prefix:expr, $ty:ty) => {
        ::summer::submit_inventory! {
            ::summer::config::ConfigSchema {
                prefix: $prefix,
                schema: || ::summer::config::schema_for!($ty),
            }
        }
    };
}

/// Get all registered schemas
pub fn auto_config_schemas() -> Vec<(String, Schema)> {
    inventory::iter::<ConfigSchema>
        .into_iter()
        .map(|c| (c.prefix.to_string(), (c.schema)()))
        .collect()
}

/// Merge all config schemas into one json schema
pub fn merge_all_schemas() -> serde_json::Value {
    let mut properties = serde_json::Map::new();

    for (prefix, schema) in auto_config_schemas() {
        // Put each schema under the corresponding prefix
        properties.insert(prefix, serde_json::to_value(schema).unwrap());
    }

    json!({
        "type": "object",
        "properties": properties
    })
}

/// write merged json schema to file
pub fn write_merged_schema_to_file(path: &str) -> std::io::Result<()> {
    let merged = merge_all_schemas();
    std::fs::write(path, serde_json::to_string_pretty(&merged).unwrap())
}
