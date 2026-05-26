# summer-xxl-job

Integrate [`xxljob-sdk-rs`](https://crates.io/crates/xxljob-sdk-rs) with the
[summer-rs](https://github.com/summer-rs/summer-rs) framework so that an
application can act as an **executor** of an xxl-job-admin or
[ratch-job](https://github.com/ratch-job/ratch-job) compatible scheduling
server.

[简体中文](./README.zh.md)

## Configuration

```toml
# config/app.toml
[xxl-job]
admin_addresses = "http://127.0.0.1:8080/xxl-job-admin"
app_name        = "summer-xxl-executor"
access_token    = "default_token"
log_path        = "logs/xxl-job"
# optional
# ip                = "10.0.0.5"
# port              = 9999
# log_retention_days = 30
# ssl_danger_accept_invalid_certs = false
# [xxl-job.headers]
# X-Gateway-Token = "my-token"
```

## Minimum example

```rust
use summer::{async_trait, App};
use summer_xxl_job::{
    AsyncJobHandler, JobContext, XxlJobConfigurator, XxlJobPlugin,
};

pub struct DemoJobHandler;

#[async_trait]
impl AsyncJobHandler for DemoJobHandler {
    async fn process(&self, ctx: JobContext) -> anyhow::Result<JobContext> {
        tracing::info!(job_id = ctx.job_id, "running demo handler");
        Ok(ctx)
    }
}

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(XxlJobPlugin)
        .add_xxl_async_handler("demoJobHandler", DemoJobHandler)
        .run()
        .await;
}
```

## Dependency-injected handlers

If a handler needs components or configs from the Summer container, derive
[`Service`](https://docs.rs/summer/latest/summer/plugin/service/trait.Service.html)
on it and register the type via `add_xxl_async_service::<H>(name)` /
`add_xxl_sync_service::<H>(name)`. The plugin will instantiate the handler
after Summer's dependency-injection phase finishes, so `#[inject(component)]`
and `#[inject(config)]` fields are resolved correctly.

```rust
use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;
use summer::plugin::service::Service;
use summer::{async_trait, App};
use summer_xxl_job::{
    AsyncJobHandler, JobContext, SyncJobHandler, XxlClientHandle,
    XxlJobConfigurator, XxlJobPlugin,
};

#[derive(Debug, Clone, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "demo-job"]
pub struct DemoJobConfig {
    pub greeting: String,
}

// Async handler with DI
#[derive(Clone, Service)]
pub struct DemoServiceHandler {
    #[inject(component)]
    xxl_client: XxlClientHandle,
    #[inject(config)]
    cfg: DemoJobConfig,
}

#[async_trait]
impl AsyncJobHandler for DemoServiceHandler {
    async fn process(&self, ctx: JobContext) -> anyhow::Result<JobContext> {
        tracing::info!(greeting = %self.cfg.greeting, "async DI handler");
        Ok(ctx)
    }
}

// Sync handler with DI (good for CPU-bound / blocking work)
#[derive(Clone, Service)]
pub struct DemoSyncServiceHandler {
    #[inject(config)]
    cfg: DemoJobConfig,
}

impl SyncJobHandler for DemoSyncServiceHandler {
    fn process(&self, ctx: JobContext) -> anyhow::Result<JobContext> {
        tracing::info!(greeting = %self.cfg.greeting, "sync DI handler");
        Ok(ctx)
    }
}

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(XxlJobPlugin)
        .add_xxl_async_service::<DemoServiceHandler>("demoServiceJobHandler")
        .add_xxl_sync_service::<DemoSyncServiceHandler>("demoSyncServiceJobHandler")
        .run()
        .await;
}
```

### Handler registration APIs

| API | Handler trait | Constructed | Use case |
|-----|---------------|-------------|----------|
| `add_xxl_async_handler(name, value)` | `AsyncJobHandler` | Eagerly at call site | Stateless / pre-built handlers |
| `add_xxl_sync_handler(name, value)` | `SyncJobHandler`  | Eagerly at call site | Stateless / pre-built handlers |
| `add_xxl_async_service::<H>(name)`   | `AsyncJobHandler` + `Service` | Lazily after DI is ready | Async handlers that need `#[inject(...)]` fields |
| `add_xxl_sync_service::<H>(name)`    | `SyncJobHandler`  + `Service` | Lazily after DI is ready | Sync handlers that need `#[inject(...)]` fields |

## Coexistence with `summer-job`

`summer-xxl-job` only handles **remote** scheduling driven by
xxl-job-admin / ratch-job. Local cron / fixed-rate / fixed-delay jobs are
still provided by [`summer-job`](../summer-job). The two plugins are
independent and can be enabled at the same time.

## Features

| Feature      | Effect                                          |
|--------------|-------------------------------------------------|
| `rustls-tls` | Enable HTTPS to admin via `reqwest`'s `rustls`. |
| `native-tls` | Enable HTTPS to admin via `reqwest`'s native TLS backend. |
