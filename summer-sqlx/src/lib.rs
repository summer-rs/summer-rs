//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-sqlx)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;
pub extern crate sqlx;
use anyhow::Context;
use config::SqlxConfig;
use summer::app::AppBuilder;
use summer::config::ConfigRegistry;
use summer::error::Result;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry, Plugin};
use summer::{async_trait, App};
use sqlx::{Database, Pool};
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(feature = "postgres"))]
pub type ConnectPool = sqlx::AnyPool;
#[cfg(feature = "postgres")]
pub type ConnectPool = sqlx::PgPool;

pub struct SqlxPlugin;

#[async_trait]
impl Plugin for SqlxPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        sqlx::any::install_default_drivers();
        let config = app
            .get_config::<SqlxConfig>()
            .expect("sqlx plugin config load failed");

        let connect_pool = Self::connect(&config)
            .await
            .expect("sqlx plugin load failed");

        tracing::info!("sqlx connection success");

        app.add_component(connect_pool)
            .add_shutdown_hook(|app| Box::new(Self::close_db_connection(app)));
    }
}

impl SqlxPlugin {
    #[cfg(not(feature = "postgres"))]
    pub async fn connect(config: &SqlxConfig) -> Result<ConnectPool> {
        use sqlx::any::AnyPoolOptions;

        let opt = Self::configure_pool(AnyPoolOptions::new(), config);
        Self::establish_connection(opt, &config.uri).await
    }

    #[cfg(feature = "postgres")]
    pub async fn connect(config: &SqlxConfig) -> Result<ConnectPool> {
        use sqlx::postgres::PgPoolOptions;

        let opt = Self::configure_pool(PgPoolOptions::new(), config);
        Self::establish_connection(opt, &config.uri).await
    }

    fn configure_pool<T>(
        mut opt: sqlx::pool::PoolOptions<T>,
        config: &SqlxConfig,
    ) -> sqlx::pool::PoolOptions<T>
    where
        T: Database,
    {
        opt = opt
            .max_connections(config.max_connections)
            .min_connections(config.min_connections);

        if let Some(acquire_timeout) = config.acquire_timeout {
            opt = opt.acquire_timeout(Duration::from_millis(acquire_timeout));
        }
        if let Some(idle_timeout) = config.idle_timeout {
            opt = opt.idle_timeout(Duration::from_millis(idle_timeout));
        }
        if let Some(connect_timeout) = config.connect_timeout {
            opt = opt.max_lifetime(Duration::from_millis(connect_timeout));
        }

        opt
    }

    async fn establish_connection<T>(opt: sqlx::pool::PoolOptions<T>, uri: &str) -> Result<Pool<T>>
    where
        T: Database,
    {
        Ok(opt
            .connect(uri)
            .await
            .with_context(|| format!("Failed to connect to database: {uri}"))?)
    }

    async fn close_db_connection(app: Arc<App>) -> Result<String> {
        app.get_component::<ConnectPool>()
            .expect("sqlx connect pool not exists")
            .close()
            .await;
        Ok("sqlx connection pool close successful".into())
    }
}
