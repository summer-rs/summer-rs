# summer-xxl-job

将 [`xxljob-sdk-rs`](https://crates.io/crates/xxljob-sdk-rs) 集成到
[summer-rs](https://github.com/summer-rs/summer-rs) 框架，让应用作为
xxl-job-admin / [ratch-job](https://github.com/ratch-job/ratch-job)
等兼容服务端的**执行器**接入分布式任务调度。

[English](./README.md)

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

## 与 `summer-job` 的关系

`summer-xxl-job` 只处理由 xxl-job-admin / ratch-job 远程下发的调度任务；
本地的 cron / 固定速率 / 固定延迟任务仍由 [`summer-job`](../summer-job) 负责。
两个插件互不依赖，可以在同一应用中同时启用。

## Feature 开关

| Feature      | 作用                                          |
|--------------|-----------------------------------------------|
| `rustls-tls` | 通过 `reqwest` 的 rustls 后端访问 HTTPS admin |
| `native-tls` | 通过 `reqwest` 的 native-tls 后端访问 HTTPS admin |
