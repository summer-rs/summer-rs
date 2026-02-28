//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-sea-orm)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;
pub mod pagination;

use anyhow::Context;
use config::SeaOrmConfig;
use sea_orm::{ConnectOptions, Database};
use summer::config::ConfigRegistry;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry};
use summer::{app::AppBuilder, error::Result, plugin::Plugin};
use summer::{async_trait, App};
use std::sync::Arc;
use std::time::Duration;

pub type DbConn = sea_orm::DbConn;

pub struct SeaOrmPlugin;

#[async_trait]
impl Plugin for SeaOrmPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<SeaOrmConfig>()
            .expect("sea-orm plugin config load failed");

        let conn = Self::connect(&config)
            .await
            .expect("sea-orm plugin load failed");
        app.add_component(conn)
            .add_shutdown_hook(|app| Box::new(Self::close_db_connection(app)));
    }
}

impl SeaOrmPlugin {
    pub async fn connect(config: &config::SeaOrmConfig) -> Result<DbConn> {
        let mut opt = ConnectOptions::new(&config.uri);
        opt.max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .sqlx_logging(config.enable_logging);

        if let Some(connect_timeout) = config.connect_timeout {
            opt.connect_timeout(Duration::from_millis(connect_timeout));
        }
        if let Some(idle_timeout) = config.idle_timeout {
            opt.idle_timeout(Duration::from_millis(idle_timeout));
        }
        if let Some(acquire_timeout) = config.acquire_timeout {
            opt.acquire_timeout(Duration::from_millis(acquire_timeout));
        }

        Ok(Database::connect(opt)
            .await
            .with_context(|| format!("sea-orm connection failed:{}", &config.uri))?)
    }

    async fn close_db_connection(app: Arc<App>) -> Result<String> {
        app.get_component::<DbConn>()
            .expect("sea-orm db connection not exists")
            .close()
            .await
            .context("sea-orm db connection close failed")?;
        Ok("sea-orm db connection close successful!".into())
    }
}
