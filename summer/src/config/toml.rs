use super::env::Env;
use super::{ConfigRegistry, Configurable};
use crate::error::{AppError, Result};
use anyhow::Context;
use serde::de::DeserializeOwned;
use serde_toml_merge::merge_tables;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use toml::Table;

/// Configuration management based on Toml
#[derive(Default)]
pub struct TomlConfigRegistry {
    config: Table,
}

impl ConfigRegistry for TomlConfigRegistry {
    fn get_config<T>(&self) -> Result<T>
    where
        T: DeserializeOwned + Configurable,
    {
        let prefix = T::config_prefix();
        let table = self.get_by_prefix(prefix);
        T::deserialize(table.to_owned()).map_err(|e| AppError::DeserializeErr(prefix, e))
    }
}

impl TomlConfigRegistry {
    /// Read configuration from a configuration file.
    /// If there is a configuration file corresponding to the [active environment][Env] in the same directory,
    /// the environment configuration file will be merged with the main configuration file.
    pub fn new(config_path: &Path, env: Env) -> Result<Self> {
        let config = Self::load_config(config_path, env)?;
        Ok(Self { config })
    }

    /// Get all configurations for a specified prefix
    pub fn get_by_prefix(&self, prefix: &str) -> Table {
        match self.config.get(prefix) {
            Some(toml::Value::Table(table)) => table.clone(),
            _ => Table::new(),
        }
    }

    /// load toml config
    fn load_config(config_path: &Path, env: Env) -> Result<Table> {
        let config_file_content = fs::read_to_string(config_path);
        let main_toml_str = match config_file_content {
            Err(e) => {
                log::warn!("Failed to read configuration file {config_path:?}: {e}");
                return Ok(Table::new());
            }
            Ok(content) => super::env::interpolate(&content),
        };

        let main_table = toml::from_str::<Table>(main_toml_str.as_str())
            .with_context(|| format!("Failed to parse the toml file at path {config_path:?}"))?;

        let config_table: Table = match env.get_config_path(config_path) {
            Ok(env_path) => {
                let env_path = env_path.as_path();
                if !env_path.exists() {
                    return Ok(main_table);
                }
                log::info!("The profile of the {env:?} environment is active");

                let env_toml_str = fs::read_to_string(env_path)
                    .with_context(|| format!("Failed to read configuration file {env_path:?}"))?;
                let env_toml_str = super::env::interpolate(&env_toml_str);
                let env_table =
                    toml::from_str::<Table>(env_toml_str.as_str()).with_context(|| {
                        format!("Failed to parse the toml file at path {env_path:?}")
                    })?;
                merge_tables(main_table, env_table)
                    .map_err(|e| AppError::TomlMergeError(e.to_string()))
                    .with_context(|| {
                        format!("Failed to merge files {config_path:?} and {env_path:?}")
                    })?
            }
            Err(_) => {
                log::debug!("{env:?} config not found");
                main_table
            }
        };

        Ok(config_table)
    }
}

impl FromStr for TomlConfigRegistry {
    type Err = AppError;

    fn from_str(str: &str) -> std::result::Result<Self, Self::Err> {
        let config = toml::from_str::<Table>(str)?;
        Ok(Self { config })
    }
}

#[cfg(test)]
mod tests {
    use super::Env;
    use super::TomlConfigRegistry;
    use crate::error::Result;
    use std::fs;

    #[test]
    fn test_load_config() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;

        let foo = temp_dir.path().join("foo.toml");
        #[rustfmt::skip]
        let _ = fs::write(&foo,r#"
        [group]
        key = "A"
        "#,
        );

        let table = TomlConfigRegistry::new(&foo, Env::from_string("dev"))?;
        let group = table.get_by_prefix("group");
        assert_eq!(group.get("key").unwrap().as_str(), Some("A"));

        // test merge
        let foo_dev = temp_dir.path().join("foo-dev.toml");
        #[rustfmt::skip]
        let _ = fs::write(foo_dev,r#"
        [group]
        key = "OOOOA"
        "#,
        );

        let table = TomlConfigRegistry::new(&foo, Env::from_string("dev"))?;
        let group = table.get_by_prefix("group");
        assert_eq!(group.get("key").unwrap().as_str(), Some("OOOOA"));

        Ok(())
    }
}
