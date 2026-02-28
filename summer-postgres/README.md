[![crates.io](https://img.shields.io/crates/v/spring-postgres.svg)](https://crates.io/crates/spring-postgres)
[![Documentation](https://docs.rs/spring-postgres/badge.svg)](https://docs.rs/spring-postgres)

[tokio-postgres](https://github.com/sfackler/rust-postgres) is a database connection tool similar to sqlx. Unlike sqlx, it only focuses on implementing postgresql database connections.

## Dependencies

```toml
spring-postgres = { version = "<version>" }
```

optional features:
* `array-impls`
* `js`
* `with-bit-vec-0_6`
* `with-chrono-0_4`
* `with-eui48-0_4`
* `with-eui48-1`
* `with-geo-types-0_6`
* `with-geo-types-0_7`
* `with-serde_json-1`
* `with-smol_str-01`
* `with-time-0_2`
* `with-time-0_3`
* `with-uuid-0_8`
* `with-uuid-1`

## Configuration items

```toml
[postgres]
connect = "postgres://root:12341234@localhost:5432/myapp_development" # Database address to connect to
```

## Components

After configuring the above configuration items, the plugin will automatically register a [`Postgres`](https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.Client.html) object. This object wraps [`tokio_postgres::Client`](https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.Client.html).

```rust
pub struct Postgres(Arc<tokio_postgres::Client>);
```

## Extract the Component registered by the plugin

The `PgPlugin` plugin automatically registers a [`Postgres`](https://docs.rs/tokio-postgres/latest/tokio_postgres/struct.Client.html) object for us. We can use `Component` to extract this connection pool from AppState. [`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html) is an axum [extractor](https://docs.rs/axum/latest/axum/extract/index.html).

```rust
#[get("/postgres")]
async fn hello_postgres(Component(pg): Component<Postgres>) -> Result<impl IntoResponse> {
    let rows = pg
        .query("select version() as version", &[])
        .await
        .context("query postgresql failed")?;

    let version: String = rows[0].get("version");

    Ok(Json(version))
}
```

Complete code reference [`postgres-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/postgres-example)