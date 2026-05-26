use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;
use summer::plugin::service::Service;
use summer::{async_trait, App};
use summer_xxl_job::{
    AsyncJobHandler, JobContext, SyncJobHandler, XxlClientHandle, XxlJobConfigurator, XxlJobPlugin,
};

/// An async executor handler. The `JobHandler` value `demoJobHandler`
/// configured on admin will dispatch invocations here.
pub struct DemoAsyncHandler;

#[async_trait]
impl AsyncJobHandler for DemoAsyncHandler {
    async fn process(&self, ctx: JobContext) -> anyhow::Result<JobContext> {
        tracing::info!(
            job_id = ctx.job_id,
            log_id = ctx.log_id,
            param = ?ctx.job_param,
            "async demo handler running"
        );
        for i in 0..3 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            tracing::info!(step = i, "async demo step");
        }
        Ok(ctx)
    }
}

/// A sync executor handler. Use `JobHandler::Sync(...)` for CPU-bound work.
pub struct DemoSyncHandler;

impl SyncJobHandler for DemoSyncHandler {
    fn process(&self, ctx: JobContext) -> anyhow::Result<JobContext> {
        tracing::info!(job_id = ctx.job_id, "sync demo handler running");
        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(ctx)
    }
}

/// Custom config consumed by [`DemoServiceHandler`] via `#[inject(config)]`.
#[derive(Debug, Clone, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "demo-job"]
pub struct DemoJobConfig {
    #[serde(default = "default_greeting")]
    pub greeting: String,
}

fn default_greeting() -> String {
    "hello from DI handler".to_string()
}

/// A Service-style async handler that participates in DI:
/// - `xxl_client` is injected from the component registry (registered by `XxlJobPlugin`);
/// - `cfg` is loaded from the `demo-job` section of `config/app.toml`.
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
        tracing::info!(
            job_id = ctx.job_id,
            log_id = ctx.log_id,
            greeting = %self.cfg.greeting,
            xxl_client_refs = std::sync::Arc::strong_count(&self.xxl_client.0),
            "DI service handler running"
        );
        Ok(ctx)
    }
}

/// A Service-style **sync** handler that participates in DI as well.
/// Use this style for CPU-bound jobs where each invocation runs on its own
/// blocking thread (no `async fn` allowed in the body).
#[derive(Clone, Service)]
pub struct DemoSyncServiceHandler {
    #[inject(config)]
    cfg: DemoJobConfig,
    #[inject(component)]
    xxl_client: XxlClientHandle,
}

impl SyncJobHandler for DemoSyncServiceHandler {
    fn process(&self, ctx: JobContext) -> anyhow::Result<JobContext> {
        tracing::info!(
            job_id = ctx.job_id,
            log_id = ctx.log_id,
            greeting = %self.cfg.greeting,
            xxl_client_refs = std::sync::Arc::strong_count(&self.xxl_client.0),
            "DI sync service handler running"
        );
        // Simulate CPU-bound / blocking work.
        std::thread::sleep(std::time::Duration::from_secs(1));
        Ok(ctx)
    }
}

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(XxlJobPlugin)
        .add_xxl_async_handler("demoJobHandler", DemoAsyncHandler)
        .add_xxl_sync_handler("demoSyncJobHandler", DemoSyncHandler)
        .add_xxl_async_service::<DemoServiceHandler>("demoServiceJobHandler")
        .add_xxl_sync_service::<DemoSyncServiceHandler>("demoSyncServiceJobHandler")
        .run()
        .await;
}
