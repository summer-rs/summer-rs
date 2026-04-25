mod controllers;
mod consumers;

use summer::{App, auto_config};
use summer_pubsub::PubSubPlugin;
use summer_sqlx::SqlxPlugin;
use summer_web::{WebConfigurator, WebPlugin};

#[auto_config(WebConfigurator, PubSubConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(PubSubPlugin)
        .add_plugin(SqlxPlugin)
        .run().await;
}
