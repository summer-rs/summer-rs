//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-postgres)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;
pub extern crate tokio_postgres as postgres;

use config::PgConfig;
use summer::app::AppBuilder;
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::plugin::{MutableComponentRegistry, Plugin};
use std::sync::Arc;
use tokio_postgres::NoTls;

pub type Postgres = Arc<tokio_postgres::Client>;

pub struct PgPlugin;

#[async_trait]
impl Plugin for PgPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<PgConfig>()
            .expect("postgres plugin config load failed");

        let (client, connection) = tokio_postgres::connect(&config.connect, NoTls)
            .await
            .expect("connect postgresql failed");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("postgresql connection error: {}", e);
            }
        });

        app.add_component(Postgres::new(client));
    }
}
