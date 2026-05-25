use summer::{async_trait, App};
use summer_xxl_job::{
    AsyncJobHandler, JobContext, SyncJobHandler, XxlJobConfigurator, XxlJobPlugin,
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

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(XxlJobPlugin)
        .add_xxl_async_handler("demoJobHandler", DemoAsyncHandler)
        .add_xxl_sync_handler("demoSyncJobHandler", DemoSyncHandler)
        .run()
        .await;
}
