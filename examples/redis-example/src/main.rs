use anyhow::Context;
use summer::{auto_config, App};
use summer_redis::{cache, redis::AsyncCommands, Redis, RedisPlugin};
use summer_web::{
    axum::response::{IntoResponse, Json},
    error::Result,
    extractor::{Component, Path},
    WebConfigurator, WebPlugin,
};
use summer_web::{get, post};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(RedisPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/")]
async fn list_redis_key(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let keys: Vec<String> = redis.keys("*").await.context("redis request failed")?;
    Ok(Json(keys))
}

#[get("/get/{key}")]
async fn get_content(
    Component(mut redis): Component<Redis>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse> {
    let v: String = redis.get(key).await.context("redis request failed")?;
    Ok(v)
}

#[post("/put/{key}")]
async fn set_content(
    Component(mut redis): Component<Redis>,
    Path(key): Path<String>,
    body: String,
) -> Result<impl IntoResponse> {
    let v: String = redis.set(key, body).await.context("redis request failed")?;
    Ok(v)
}

#[get("/cache/{key}")]
async fn test_cache(Path(key): Path<String>) -> Result<impl IntoResponse> {
    Ok(cachable_func(&key).await)
}

#[cache(
    "redis-cache:{key}",
    expire = 60,
    condition = key.len() > 3,
    unless = result.len() > 30
)]
async fn cachable_func(key: &str) -> String {
    println!("cachable_func called with key: {}", key);
    format!("cached value for key: {key}")
}
