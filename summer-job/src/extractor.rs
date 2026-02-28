use crate::{JobId, JobScheduler};
use serde::de::DeserializeOwned;
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::extractor::{Component, Config};
use summer::plugin::ComponentRegistry;
use summer::{app::App, config::Configurable};

#[async_trait]
pub trait FromApp {
    async fn from_app(job_id: &JobId, scheduler: &JobScheduler, app: &App) -> Self;
}

#[async_trait]
impl<T> FromApp for Component<T>
where
    T: Clone + Send + Sync + 'static,
{
    async fn from_app(_job_id: &JobId, _scheduler: &JobScheduler, app: &App) -> Self {
        match app.get_component_ref::<T>() {
            Some(component) => Component(T::clone(&component)),
            None => panic!(
                "There is no component of `{}` type",
                std::any::type_name::<T>()
            ),
        }
    }
}

#[async_trait]
impl<T> FromApp for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
{
    async fn from_app(_job_id: &JobId, _scheduler: &JobScheduler, app: &App) -> Self {
        match app.get_config::<T>() {
            Ok(config) => Config(config),
            Err(e) => panic!(
                "get config for typeof {} failed: {}",
                std::any::type_name::<T>(),
                e
            ),
        }
    }
}

#[async_trait]
impl FromApp for JobId {
    async fn from_app(job_id: &JobId, _scheduler: &JobScheduler, _app: &App) -> Self {
        *job_id
    }
}

#[async_trait]
impl FromApp for JobScheduler {
    async fn from_app(_job_id: &JobId, scheduler: &JobScheduler, _app: &App) -> Self {
        scheduler.clone()
    }
}
pub struct Data<T: DeserializeOwned>(pub T);

#[async_trait]
impl<T: DeserializeOwned> FromApp for Data<T> {
    async fn from_app(job_id: &JobId, scheduler: &JobScheduler, _app: &App) -> Self {
        let mut guard = scheduler.context.metadata_storage.write().await;
        let job = guard.get(*job_id).await.expect("job get failed");
        let data =
            job.map(|j| serde_json::from_slice(&j.extra).expect("job extra parse to json failed"));
        Self(data.expect("job extra parse is empty"))
    }
}
