//! Multi-datasource plugin example
//!
//! This module demonstrates how to configure multiple database connections
//! using tuple structs to wrap different DbConn instances.

use anyhow::Context;
use serde::Deserialize;
use summer::app::AppBuilder;
use summer::config::{ConfigRegistry, Configurable};
use summer::error::Result;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry, Plugin};
use summer::{async_trait, tracing, App};
use summer_sea_orm::config::SeaOrmConfig;
use summer_sea_orm::{DbConn, SeaOrmPlugin};
use std::ops::Deref;
use std::sync::Arc;

/// Primary database connection wrapper
/// Use this for your main/primary database
#[derive(Clone)]
pub struct PrimaryDb(pub DbConn);

impl Deref for PrimaryDb {
    type Target = DbConn;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Secondary database connection wrapper
/// Use this for your secondary/replica database
#[derive(Clone)]
pub struct SecondaryDb(pub DbConn);

impl Deref for SecondaryDb {
    type Target = DbConn;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Multi-datasource configuration
/// Reuses SeaOrmConfig from summer-sea-orm for each datasource
#[derive(Debug, Clone, Configurable, Deserialize)]
#[config_prefix = "multi-datasource"]
pub struct MultiDatasourceConfig {
    pub primary: SeaOrmConfig,
    pub secondary: SeaOrmConfig,
}

/// Multi-datasource plugin
///
/// This plugin initializes two database connections (primary and secondary)
/// and registers them as separate components that can be injected into handlers.
pub struct MultiDatasourcePlugin;

#[async_trait]
impl Plugin for MultiDatasourcePlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<MultiDatasourceConfig>()
            .expect("multi-datasource plugin config load failed");

        // Connect to primary database
        let primary_conn = SeaOrmPlugin::connect(&config.primary)
            .await
            .expect("primary database connection failed");
        tracing::info!("Primary database connection established");

        // Connect to secondary database
        let secondary_conn = SeaOrmPlugin::connect(&config.secondary)
            .await
            .expect("secondary database connection failed");
        tracing::info!("Secondary database connection established");

        // Register both connections as components
        app.add_component(PrimaryDb(primary_conn))
            .add_component(SecondaryDb(secondary_conn))
            .add_shutdown_hook(|app| Box::new(Self::close_connections(app)));
    }
}

impl MultiDatasourcePlugin {
    async fn close_connections(app: Arc<App>) -> Result<String> {
        app.get_component::<PrimaryDb>()
            .expect("primary db connection not exists")
            .0
            .close()
            .await
            .context("primary db close failed")?;

        app.get_component::<SecondaryDb>()
            .expect("secondary db connection not exists")
            .0
            .close()
            .await
            .context("secondary db close failed")?;

        Ok("All database connections closed successfully".into())
    }
}
