# summer-xxl-job

将 [`xxljob-sdk-rs`](https://crates.io/crates/xxljob-sdk-rs) 集成到
[summer-rs](https://github.com/summer-rs/summer-rs) 框架，让应用作为
xxl-job-admin / [ratch-job](https://github.com/ratch-job/ratch-job)
等兼容服务端的**执行器**接入分布式任务调度。

## 配置

```toml
# config/app.toml
[xxl-job]
admin_addresses = "http://127.0.0.1:8080/xxl-job-admin"
app_name        = "summer-xxl-executor"
access_token    = "default_token"
log_path        = "logs/xxl-job"
# 可选项
# ip                = "10.0.0.5"
# port              = 9999
# log_retention_days = 30
# ssl_danger_accept_invalid_certs = false
# [xxl-job.headers]
# X-Gateway-Token = "my-token"
```

## 最小示例

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

## 依赖注入版 Handler

如果 Handler 需要从 Summer 容器获取组件或配置，可以为它派生
[`Service`](https://docs.rs/summer/latest/summer/plugin/service/trait.Service.html)，
再通过 `add_xxl_async_service::<H>(name)` / `add_xxl_sync_service::<H>(name)`
注册。插件会在 Summer 完成依赖注入阶段之后再实例化 Handler，因此
`#[inject(component)]` / `#[inject(config)]` 等字段都能被正确解析。

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

// 带 DI 的异步 Handler
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

// 带 DI 的同步 Handler（适合 CPU 密集 / 阻塞型任务）
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

### Handler 注册接口对比

| API | Handler trait | 实例化时机 | 适用场景 |
|-----|---------------|-----------|---------|
| `add_xxl_async_handler(name, value)` | `AsyncJobHandler` | 调用处立即构造 | 无依赖、已构造好的 Handler |
| `add_xxl_sync_handler(name, value)`  | `SyncJobHandler`  | 调用处立即构造 | 无依赖、已构造好的 Handler |
| `add_xxl_async_service::<H>(name)`   | `AsyncJobHandler` + `Service` | DI 完成后惰性构造 | 需要 `#[inject(...)]` 字段的异步 Handler |
| `add_xxl_sync_service::<H>(name)`    | `SyncJobHandler`  + `Service` | DI 完成后惰性构造 | 需要 `#[inject(...)]` 字段的同步 Handler |

## 与 `summer-job` 的关系

`summer-xxl-job` 只处理由 xxl-job-admin / ratch-job 远程下发的调度任务；
本地的 cron / 固定速率 / 固定延迟任务仍由 [`summer-job`](../summer-job) 负责。
两个插件互不依赖，可以在同一应用中同时启用。

## Feature 开关

| Feature      | 作用                                          |
|--------------|-----------------------------------------------|
| `rustls-tls` | 通过 `reqwest` 的 rustls 后端访问 HTTPS admin |
| `native-tls` | 通过 `reqwest` 的 native-tls 后端访问 HTTPS admin |
