use serde::{Deserialize, Serialize};
use summer::{
    app::AppBuilder,
    auto_config,
    plugin::{ComponentRegistry, MutableComponentRegistry},
    tracing, App,
};
use summer_apalis::apalis_board::axum::{
    framework::{ApiBuilder, RegisterRoute},
    sse::{TracingBroadcaster, TracingSubscriber},
    ui::ServeUI,
};
use summer_apalis::apalis_redis::RedisStorage;
use summer_apalis::{apalis::prelude::*, ApalisConfigurator as _, ApalisPlugin};
use summer_redis::{Redis, RedisPlugin};
use summer_web::{
    axum::{response::IntoResponse, Extension, Router},
    extractor::Component,
    get, WebConfigurator, WebPlugin,
};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct LongRunningJob {}

async fn long_running_task(_task: LongRunningJob, worker: WorkerContext) {
    loop {
        tracing::info!("is_shutting_down: {}", worker.is_shutting_down());
        if worker.is_shutting_down() {
            tracing::info!("saving the job state");
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await; // Do some hard thing
    }
    tracing::info!("Shutdown complete!");
}

fn long_running_task_register(app: &mut AppBuilder, monitor: Monitor) -> Monitor {
    let redis = app.get_expect_component::<Redis>();
    let storage = RedisStorage::new(redis);
    app.add_component(storage.clone());

    let broadcaster = TracingBroadcaster::create();
    let line_sub = TracingSubscriber::new(&broadcaster);
    app.add_layer(line_sub.layer());

    let api = ApiBuilder::new(Router::new())
        .register(storage.clone())
        .build();
    let router = Router::new()
        .nest("/api/v1", api)
        .fallback_service(ServeUI::new())
        .layer(Extension(broadcaster.clone()));
    app.add_router(router.into());

    monitor.register(move |_| {
        let storage = storage.clone();
        WorkerBuilder::new("long_running")
            .backend(storage)
            .enable_tracing()
            .catch_panic()
            .concurrency(2)
            .rate_limit(5, Duration::from_secs(1))
            .build(long_running_task)
    })
}

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(RedisPlugin)
        .add_plugin(WebPlugin)
        .add_plugin(ApalisPlugin)
        .add_worker(long_running_task_register)
        .run()
        .await
}

#[get("/start")]
pub async fn start_job(
    Component(mut storage): Component<RedisStorage<LongRunningJob>>,
) -> impl IntoResponse {
    storage.push(LongRunningJob {}).await.unwrap();
}
