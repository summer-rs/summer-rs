//! End-to-end smoke test for [`summer_test::MockWebPlugin`].
//!
//! Validates the chained API contract documented in the crate root:
//!
//! ```text
//! App::new()
//!     .add_plugin(MockWebPlugin)
//!     .add_router(api)
//!     .build().await?
//!     .get_expect_component::<MockServer>()
//!     .get("/ping").await
//!     .assert_status_ok();
//! ```
//!
//! Database-backed cases (`*_with_postgres_*`, `*_with_redis_*`) spin up a
//! container via `testcontainers` and inject the connection string through
//! `.use_config_str`, so they are gated behind `#[ignore]` and
//! only run with `cargo test -- --ignored` on a host with a working Docker
//! daemon.

use summer::app::App;
use summer::error::Result;
use summer::plugin::ComponentRegistry;
use summer_macros::get;
use summer_test::{MockServer, MockWebPlugin};
use summer_web::axum::routing;
use summer_web::extractor::Component;
use summer_web::{Router, WebConfigurator};

async fn ping() -> &'static str {
    "pong"
}

async fn echo(body: String) -> String {
    body
}

/// Handler installed via the `#[get]` proc-macro; picked up by
/// [`summer_web::handler::auto_router`] through `inventory`.
#[get("/typed_ping")]
async fn typed_ping() -> &'static str {
    "typed_pong"
}

#[tokio::test]
async fn mock_web_plugin_serves_get() -> Result<()> {
    App::new()
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/ping", routing::get(ping)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/ping")
        .await
        .assert_status_ok();

    Ok(())
}

#[tokio::test]
async fn mock_web_plugin_serves_post_echo() -> Result<()> {
    let response = App::new()
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/echo", routing::post(echo)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .post("/echo")
        .text("hello")
        .await;

    response.assert_status_ok();
    response.assert_text("hello");
    Ok(())
}

#[tokio::test]
async fn mock_web_plugin_returns_404_for_unknown_route() -> Result<()> {
    App::new()
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/ping", routing::get(ping)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/missing")
        .await
        .assert_status_not_found();

    Ok(())
}

#[tokio::test]
async fn mock_web_plugin_serves_typed_get_macro() -> Result<()> {
    let response = App::new()
        .add_plugin(MockWebPlugin)
        .add_router(summer_web::handler::auto_router())
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/typed_ping")
        .await;

    response.assert_status_ok();
    response.assert_text("typed_pong");
    Ok(())
}

// ----------------------------- Postgres -----------------------------

async fn pg_version(Component(pg): Component<summer_postgres::Postgres>) -> String {
    let row = pg
        .query_one("SELECT version()", &[])
        .await
        .expect("SELECT version() failed");
    row.get::<_, String>(0)
}

#[tokio::test]
async fn mock_web_plugin_with_postgres_container() -> Result<()> {
    use summer_postgres::PgPlugin;
    use testcontainers_modules::postgres::Postgres as PgImage;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = PgImage::default()
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("start postgres container failed: {e}"))?;
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .map_err(|e| anyhow::anyhow!("get postgres host port failed: {e}"))?;

    let toml = format!(
        r#"
[postgres]
connect = "postgres://postgres:postgres@127.0.0.1:{port}/postgres"
"#
    );

    let response = App::new()
        .use_config_str(&toml)
        .add_plugin(PgPlugin)
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/pg_version", routing::get(pg_version)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/pg_version")
        .await;

    response.assert_status_ok();
    assert!(
        response.text().to_lowercase().contains("postgresql"),
        "unexpected version body: {}",
        response.text()
    );
    Ok(())
}

// ------------------------------ Redis -------------------------------

async fn redis_roundtrip(mut redis: Component<summer_redis::Redis>) -> String {
    use summer_redis::redis::AsyncCommands;
    let _: () = redis
        .set("summer_test:key", "hello")
        .await
        .expect("redis SET failed");
    redis
        .get::<_, String>("summer_test:key")
        .await
        .expect("redis GET failed")
}

#[tokio::test]
async fn mock_web_plugin_with_redis_container() -> Result<()> {
    use summer_redis::RedisPlugin;
    use testcontainers_modules::redis::Redis as RedisImage;
    use testcontainers_modules::testcontainers::runners::AsyncRunner;

    let container = RedisImage::default()
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("start redis container failed: {e}"))?;
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .map_err(|e| anyhow::anyhow!("get redis host port failed: {e}"))?;

    let toml = format!(
        r#"
[redis]
uri = "redis://127.0.0.1:{port}/"
"#
    );

    let response = App::new()
        .use_config_str(&toml)
        .add_plugin(RedisPlugin)
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/redis_roundtrip", routing::get(redis_roundtrip)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/redis_roundtrip")
        .await;

    response.assert_status_ok();
    response.assert_text("hello");
    Ok(())
}
