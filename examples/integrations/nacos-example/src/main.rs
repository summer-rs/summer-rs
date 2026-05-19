use anyhow::Context;
use summer::{auto_config, App};
use summer_nacos::{NacosConfigService, NacosPlugin};
use summer_web::{
    axum::response::IntoResponse, extractor::Component, get, WebConfigurator, WebPlugin,
};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(NacosPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await;
}

#[get("/")]
async fn hello(Component(config): Component<NacosConfigService>) -> impl IntoResponse {
    let content = config
        .get_config("app.toml".to_string(), "DEFAULT_GROUP".to_string())
        .await
        .context("fetch nacos config")
        .and_then(|r| Ok(r.content().to_string()))
        .unwrap_or_else(|e| format!("config error: {e}"));
    format!("hello from nacos-example\n\n--- app.toml ---\n{content}")
}
