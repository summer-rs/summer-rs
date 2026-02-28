<div align="center">
    <img src="https://raw.githubusercontent.com/summer-rs/summer-rs/refs/heads/master/docs/static/logo-rust.svg" alt="Logo" width="200"/>
    <h3>summer-rs is application framework written in Rust, inspired by Java's SpringBoot</h3>
    <p>English ｜ <a href="https://summer-rs.github.io/zh/docs/getting-started/introduction/">中文</a></p>
    <p>
        <a href="https://crates.io/crates/summer"><img src="https://img.shields.io/crates/v/summer.svg" alt="crates.io"/></a> <a href="https://docs.rs/summer"><img src="https://docs.rs/summer/badge.svg" alt="Documentation"/></a> <img src="https://img.shields.io/crates/l/summer" alt="Documentation"/>
    </p>
</div>

<b>summer-rs</b> is an application framework that emphasizes convention over configuration, inspired by Java's SpringBoot. <b>summer-rs</b> provides an easily extensible plug-in system for integrating excellent projects in the Rust community, such as axum, sqlx, sea-orm, etc.

Compared with SpringBoot in java, summer-rs has higher performance and lower memory usage, allowing you to completely get rid of the bloated JVM and travel light.

## Features

* ⚡️ High performance: Benefiting from the awesome rust language, <b>summer-rs</b> has the ultimate performance comparable to C/C++
* 🛡️ High security: Compared to C/C++, the Rust language used by <b>summer-rs</b> provides memory safety and thread safety.
* 🔨 Lightweight: The core code of summer-rs does not exceed 5,000 lines, and the binary size of the release version packaged in rust is also small.
* 🔧 Easy to use: <b>summer-rs</b> provides a clear and concise API and optional Procedural Macros to simplify development.
* 🔌 Highly extensible: <b>summer-rs</b> uses a highly extensible plug-in model, and users can customize plug-ins to extend program capabilities.
* ⚙️ Highly configurable: <b>summer-rs</b> uses toml to configure applications and plug-ins to improve application flexibility.

## Example

**web**

```rust,no_run
use summer::{auto_config, App};
use summer_sqlx::{
    sqlx::{self, Row},
    ConnectPool, SqlxPlugin
};
use summer_web::{get, route};
use summer_web::{
    error::Result, extractor::{Path, Component}, handler::TypeRouter, axum::response::IntoResponse, Router,
    WebConfigurator, WebPlugin,
};
use anyhow::Context;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/")]
async fn hello_world() -> impl IntoResponse {
    "hello world"
}

#[route("/hello/{name}", method = "GET", method = "POST")]
async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    format!("hello {name}")
}

#[get("/version")]
async fn sqlx_request_handler(Component(pool): Component<ConnectPool>) -> Result<String> {
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
```

**job**

```rust,ignore
use anyhow::Context;
use summer::{auto_config, App};
use summer_job::{cron, fix_delay, fix_rate};
use summer_job::{extractor::Component, JobConfigurator, JobPlugin};
use summer_sqlx::{
    sqlx::{self, Row},
    ConnectPool, SqlxPlugin,
};
use std::time::{Duration, SystemTime};

#[auto_config(JobConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(JobPlugin)
        .add_plugin(SqlxPlugin)
        .run()
        .await;

    tokio::time::sleep(Duration::from_secs(100)).await;
}

#[cron("1/10 * * * * *")]
async fn cron_job(Component(db): Component<ConnectPool>) {
    let time: String = sqlx::query("select TO_CHAR(now(),'YYYY-MM-DD HH24:MI:SS') as time")
        .fetch_one(&db)
        .await
        .context("query failed")
        .unwrap()
        .get("time");
    println!("cron scheduled: {:?}", time)
}

#[fix_delay(5)]
async fn fix_delay_job() {
    let now = SystemTime::now();
    let datetime: sqlx::types::chrono::DateTime<sqlx::types::chrono::Local> = now.into();
    let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S");
    println!("fix delay scheduled: {}", formatted_time)
}

#[fix_rate(5)]
async fn fix_rate_job() {
    tokio::time::sleep(Duration::from_secs(10)).await;
    let now = SystemTime::now();
    let datetime: sqlx::types::chrono::DateTime<sqlx::types::chrono::Local> = now.into();
    let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S");
    println!("fix rate scheduled: {}", formatted_time)
}
```

## component macros

Add dependencies to your `Cargo.toml`:

```toml
[dependencies]
summer = "0.4"
tokio = { version = "1", features = ["full"] }
```

**Simple component registration with `#[component]` macro:**

```rust,no_run
use summer::component;
use summer::config::Configurable;
use summer::extractor::Config;
use summer::plugin::ComponentRegistry;
use summer::App;
use serde::Deserialize;

// Define configuration
#[derive(Clone, Configurable, Deserialize)]
#[config_prefix = "app"]
struct AppConfig {
    name: String,
}

// Define component
#[derive(Clone)]
struct AppService {
    config: AppConfig,
}

// Use #[component] macro for automatic registration
#[component]
fn app_service(Config(config): Config<AppConfig>) -> AppService {
    AppService { config }
}

#[tokio::main]
async fn main() {
    // Components are automatically registered
    let app = App::new().build().await.unwrap();
    
    // Get registered component
    let service = app.get_component::<AppService>().unwrap();
    println!("App name: {}", service.config.name);
}
```

The `#[component]` macro eliminates boilerplate code - no need to manually implement the Plugin trait! [Learn more →](https://summer-rs.github.io/docs/getting-started/component/)

## Supported plugins

| Plugin                | Crate                                                                                                                                                                      | Integrated With                                                               | Description                                      |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------ |
| `summer-web`            | [![summer-web](https://img.shields.io/crates/v/summer-web.svg)](https://summer-rs.github.io/docs/plugins/summer-web/)                                         | [`axum`](https://github.com/tokio-rs/axum)                                  | Web framework based on Axum                      |
| `summer-sqlx`           | [![summer-sqlx](https://img.shields.io/crates/v/summer-sqlx.svg)](https://summer-rs.github.io/docs/plugins/summer-sqlx/)                                     | [`sqlx`](https://github.com/launchbadge/sqlx)                               | Async SQL access                                 |
| `summer-postgres`       | [![summer-postgres](https://img.shields.io/crates/v/summer-postgres.svg)](https://summer-rs.github.io/docs/plugins/summer-postgres/)                     | [`rust-postgres`](https://github.com/sfackler/rust-postgres)                | PostgreSQL client integration                   |
| `summer-sea-orm`        | [![summer-sea-orm](https://img.shields.io/crates/v/summer-sea-orm.svg)](https://summer-rs.github.io/docs/plugins/summer-sea-orm/)                         | [`sea-orm`](https://www.sea-ql.org/SeaORM/)                                 | ORM support                                      |
| `summer-redis`          | [![summer-redis](https://img.shields.io/crates/v/summer-redis.svg)](https://summer-rs.github.io/docs/plugins/summer-redis/)                                 | [`redis`](https://github.com/redis-rs/redis-rs)                             | Redis integration                                |
| `summer-mail`           | [![summer-mail](https://img.shields.io/crates/v/summer-mail.svg)](https://summer-rs.github.io/docs/plugins/summer-mail/)                                     | [`lettre`](https://github.com/lettre/lettre)                                | Email sending                                    |
| `summer-job`            | [![summer-job](https://img.shields.io/crates/v/summer-job.svg)](https://summer-rs.github.io/docs/plugins/summer-job/)                                         | [`tokio-cron-scheduler`](https://github.com/mvniekerk/tokio-cron-scheduler) | Scheduled jobs / Cron                            |
| `summer-stream`         | [![summer-stream](https://img.shields.io/crates/v/summer-stream.svg)](https://summer-rs.github.io/docs/plugins/summer-stream/)                             | [`sea-streamer`](https://github.com/SeaQL/sea-streamer)                     | Stream processing (Redis Streams / Kafka)       |
| `summer-opentelemetry`  | [![summer-opentelemetry](https://img.shields.io/crates/v/summer-opentelemetry.svg)](https://summer-rs.github.io/docs/plugins/summer-opentelemetry/) | [`opentelemetry`](https://github.com/open-telemetry/opentelemetry-rust)     | Logging, metrics, and distributed tracing        |
| `summer-grpc`           | [![summer-grpc](https://img.shields.io/crates/v/summer-grpc.svg)](https://summer-rs.github.io/docs/plugins/summer-grpc/)                                     | [`tonic`](https://github.com/hyperium/tonic)                                | gRPC services and clients                        |
| `summer-opendal`        | [![summer-opendal](https://img.shields.io/crates/v/summer-opendal.svg)](https://summer-rs.github.io/docs/plugins/summer-opendal/)                         | [`opendal`](https://github.com/apache/opendal)                              | Unified object storage and data access           |
| `summer-apalis`        | [![summer-apalis](https://img.shields.io/crates/v/summer-apalis.svg)](https://summer-rs.github.io/docs/plugins/summer-apalis/)                         | [`apalis`](https://github.com/apalis-dev/apalis)                              | High-performance background processing library |
| `summer-sa-token`      | [![summer-sa-token](https://img.shields.io/crates/v/summer-sa-token.svg)](https://summer-rs.github.io/docs/plugins/summer-sa-token/)               | [`sa-token-rust`](https://github.com/click33/sa-token-rust)                   | Sa-Token authentication and authorization      |

## Ecosystem

* ![summer-sqlx-migration-plugin](https://img.shields.io/crates/v/summer-sqlx-migration-plugin.svg) [`summer-sqlx-migration-plugin`](https://github.com/Phosphorus-M/summer-sqlx-migration-plugin)
* [![Version](https://img.shields.io/visual-studio-marketplace/v/holmofy.summer-rs)](https://marketplace.visualstudio.com/items?itemName=holmofy.summer-rs)[`summer-lsp`](https://github.com/summer-rs/summer-lsp) - IDE support for VSCode / compatible editor with LSP
* [![JetBrains Plugin](https://img.shields.io/badge/JetBrains-Plugin-orange)](https://plugins.jetbrains.com/plugin/30040-summer-rs) [`intellij-summer-rs`](https://github.com/ouywm/intellij-summer-rs) - IDE support for RustRover / IntelliJ IDEA

[more>>](https://crates.io/crates/summer/reverse_dependencies)

## Project showcase

* [Raline](https://github.com/ralinejs/raline)
* [AutoWDS](https://github.com/AutoWDS/autowds-backend)

## Contribution

We also welcome community experts to contribute their own plugins. [Contributing →](https://github.com/summer-rs/summer-rs)

## Help

Click here to view common problems encountered when using `summer-rs` [Help →](https://summer-rs.github.io/docs/help/faq/)
