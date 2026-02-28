[![crates.io](https://img.shields.io/crates/v/summer-sqlx.svg)](https://crates.io/crates/summer-sqlx)
[![Documentation](https://docs.rs/summer-sqlx/badge.svg)](https://docs.rs/summer-sqlx)

[SQLx](https://github.com/launchbadge/sqlx) is an asynchronous SQL library that provides strongly typed database access with zero runtime reflection by validating SQL statements at compile time, without relying on an ORM.

## Dependencies

```toml
summer-sqlx = { version = "<version>", features = ["mysql"] }
```

You can replace `postgres`, `mysql`, `sqlite`feature to select the appropriate database driver.

optional features: 
* `with-json`
* `with-chrono`
* `with-rust_decimal`
* `with-bigdecimal`
* `with-uuid`
* `with-time`

## Configuration items

```toml
[sqlx]
uri = "postgres://root:123456@localhost:5432/pg_db"  # Database address
min_connections = 1          # Minimum number of connections in the connection pool, the default value is 1
max_connections = 10         # Maximum number of connections in the connection pool, the default value is 10
acquire_timeout = 30000      # Connection timeout, in milliseconds, default 30s
idle_timeout = 600000        # Connection idle time, in milliseconds, default 10min
connect_timeout = 1800000    # Maximum connection survival time, in milliseconds, default 30min
```

## Components

After configuring the above configuration items, the plugin will automatically register a [`ConnectPool`](https://docs.rs/summer-sqlx/latest/summer_sqlx/type.ConnectPool.html) connection pool object. This object is an alias for [`sqlx::AnyPool`](https://docs.rs/sqlx/latest/sqlx/type.AnyPool.html).

```rust
pub type ConnectPool = sqlx::AnyPool;
```

## Extract the Component registered by the plugin

The `SqlxPlugin` plugin automatically registers a Sqlx connection pool component for us. We can use `Component` to extract this connection pool from AppState. [`Component`](https://docs.rs/summer-web/latest/summer_web/extractor/struct.Component.html) is an axum [extractor](https://docs.rs/axum/latest/axum/extract/index.html).

```rust
use summer_sqlx::{sqlx::{self, Row}, ConnectPool};
use summer_web::get;
use summer_web::extractor::Component;
use summer_web::error::Result;
use anyhow::Context;

#[get("/version")]
async fn mysql_version(Component(pool): Component<ConnectPool>) -> Result<String> {
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
```

Complete code reference [`sqlx-example`](https://github.com/summer-rs/summer-rs/tree/master/examples/sqlx-example)