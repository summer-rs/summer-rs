use anyhow::Context;
use serde_json::json;
use summer::{auto_config, App};
use summer_stream::{Producer, StreamPlugin};
use summer_web::error::Result;
use summer_web::get;
use summer_web::{
    axum::response::{IntoResponse, Json},
    extractor::Component,
    WebConfigurator, WebPlugin,
};
use std::time::SystemTime;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(StreamPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/")]
async fn send_msg(Component(producer): Component<Producer>) -> Result<impl IntoResponse> {
    let now = SystemTime::now();
    let json = json!({
        "success": true,
        "msg": format!("This message was sent at {:?}", now),
    });
    let resp = producer
        .send_json("topic", json)
        .await
        .context("send msg failed")?;

    let seq = resp.sequence();
    Ok(Json(json!({"seq":seq})))
}
