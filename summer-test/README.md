[![crates.io](https://img.shields.io/crates/v/summer-test.svg)](https://crates.io/crates/summer-test)
[![Documentation](https://docs.rs/summer-test/badge.svg)](https://docs.rs/summer-test)

Testing utilities for the [summer-rs](https://github.com/summer-rs/summer-rs) framework.

This crate provides a Web E2E harness built on [`axum-test`](https://docs.rs/axum-test). Use it in integration tests to exercise the same router assembly path as production without binding a TCP listener or entering the serve loop.

## Dependencies

Add as a **dev-dependency** in your application crate:

```toml
[dev-dependencies]
summer-test = { version = "<version>" }
summer-web = { version = "<version>" }
tokio = { version = "1", features = ["full", "macros"] }
```

Optional **features**:

| Feature | Default | Description |
|---------|---------|-------------|
| `web` | yes | Enables `MockWebPlugin`, `MockServer`, and the `axum_test` re-export |

## Quick start

Replace [`WebPlugin`](https://docs.rs/summer-web/latest/summer_web/struct.WebPlugin.html) with [`MockWebPlugin`](https://docs.rs/summer-test/latest/summer_test/struct.MockWebPlugin.html), call [`build`](https://docs.rs/summer/latest/summer/app/struct.AppBuilder.html#method.build) instead of [`run`](https://docs.rs/summer/latest/summer/app/struct.App.html#method.run), then send requests through the in-memory server:

```rust
use summer::app::App;
use summer::plugin::ComponentRegistry;
use summer_test::{MockServer, MockWebPlugin};
use summer_web::{Router, WebConfigurator, axum::routing};

async fn ping() -> &'static str {
    "pong"
}

#[tokio::test]
async fn ping_returns_pong() -> summer::error::Result<()> {
    App::new()
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/ping", routing::get(ping)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/ping")
        .await
        .assert_status_ok()
        .assert_text("pong");

    Ok(())
}
```

## How it works

[`MockWebPlugin`](https://docs.rs/summer-test/latest/summer_test/struct.MockWebPlugin.html) is a drop-in replacement for [`WebPlugin`](https://docs.rs/summer-web/latest/summer_web/struct.WebPlugin.html):

1. **Same router path** — Reuses [`summer_web::assemble_router`](https://docs.rs/summer-web/latest/summer_web/fn.assemble_router.html) and [`summer_web::finalize_router`](https://docs.rs/summer-web/latest/summer_web/fn.finalize_router.html), so handlers, middlewares, layers, and OpenAPI behave like runtime.
2. **In-memory server** — Wraps the finalized router in [`axum_test::TestServer`](https://docs.rs/axum-test/latest/axum_test/struct.TestServer.html) instead of binding a port.
3. **Immediate return** — No scheduler is registered; `App::new().build().await` completes without a serve loop.
4. **Plugin compatibility** — `name()` returns `"summer_web::WebPlugin"`, so plugins that depend on the production web plugin (via [`Plugin::dependencies`](https://docs.rs/summer/latest/summer/plugin/trait.Plugin.html#method.dependencies)) keep resolving correctly.
5. **Synthetic startup event** — Publishes [`ServerStartedEvent`](https://docs.rs/summer/latest/summer/event/struct.ServerStartedEvent.html) (`127.0.0.1:0`, HTTP) so listeners such as [`summer-nacos`](../summer-nacos) maintain their invariants in tests.

[`MockServer`](https://docs.rs/summer-test/latest/summer_test/struct.MockServer.html) is registered as a normal component on the built [`App`](https://docs.rs/summer/latest/summer/app/struct.App.html) and implements [`Deref<Target = TestServer>`](https://doc.rust-lang.org/std/ops/trait.Deref.html), so you can chain HTTP calls directly:

```rust
app.get_expect_component::<MockServer>()
    .post("/echo")
    .text("hello")
    .await
    .assert_status_ok();
```

The crate also re-exports [`axum_test`](https://docs.rs/axum-test) for convenience when writing assertions or custom request helpers.

## Testing with other plugins

Use `MockWebPlugin` together with any plugin you would use in production. Inject configuration via [`.use_config_str`](https://docs.rs/summer/latest/summer/app/struct.AppBuilder.html#method.use_config_str) or a test config file:

```rust
App::new()
    .use_config_str(r#"
[postgres]
connect = "postgres://postgres:postgres@127.0.0.1:5432/postgres"
"#)
    .add_plugin(PgPlugin)
    .add_plugin(MockWebPlugin)
    .add_router(router())
    .build()
    .await?;
```

For database-backed tests, [testcontainers](https://crates.io/crates/testcontainers) works well — start a container, inject the connection string through `.use_config_str`, then assert on HTTP responses. See the [`web_e2e`](tests/web_e2e.rs) tests in this crate for Postgres and Redis examples.

> **NOTE**: Container-based tests require a working Docker daemon.

## API overview

| Item | Description |
|------|-------------|
| [`MockWebPlugin`](https://docs.rs/summer-test/latest/summer_test/struct.MockWebPlugin.html) | Test replacement for `WebPlugin` |
| [`MockServer`](https://docs.rs/summer-test/latest/summer_test/struct.MockServer.html) | In-memory `TestServer` handle stored as an `App` component |
| [`axum_test`](https://docs.rs/axum-test) | Re-exported for request/response helpers and assertions |

## Running this crate's tests

From the workspace root:

```bash
cargo test -p summer-test
```

Postgres and Redis E2E tests (`mock_web_plugin_with_postgres_container`, `mock_web_plugin_with_redis_container`) require Docker.
