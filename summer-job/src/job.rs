use crate::{
    handler::{BoxedHandler, Handler},
    JobId, JobScheduler,
};
use serde::Serialize;
use summer::app::App;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
enum Trigger {
    OneShot(u64),
    FixedDelay(u64),
    FixedRate(u64),
    Cron(String),
}

#[derive(Clone)]
pub struct Job {
    trigger: Trigger,
    handler: BoxedHandler,
    extra: Option<Vec<u8>>,
}

pub struct JobBuilder<T: Serialize = ()> {
    trigger: Trigger,
    data: Option<T>,
}

impl Job {
    pub fn one_shot(delay_seconds: u64) -> JobBuilder {
        JobBuilder {
            trigger: Trigger::OneShot(delay_seconds),
            data: None,
        }
    }
    /// TODO: tokio-cron-scheduler not support: <https://github.com/mvniekerk/tokio-cron-scheduler/issues/56>
    pub fn fix_delay(seconds: u64) -> JobBuilder {
        JobBuilder {
            trigger: Trigger::FixedDelay(seconds),
            data: None,
        }
    }
    pub fn fix_rate(seconds: u64) -> JobBuilder {
        JobBuilder {
            trigger: Trigger::FixedRate(seconds),
            data: None,
        }
    }
    pub fn cron(cron: &str) -> JobBuilder {
        JobBuilder {
            trigger: Trigger::Cron(cron.to_string()),
            data: None,
        }
    }
    pub fn one_shot_with_data<T: Serialize>(delay_seconds: u64, data: T) -> JobBuilder<T> {
        JobBuilder {
            trigger: Trigger::OneShot(delay_seconds),
            data: Some(data),
        }
    }
    /// TODO: tokio-cron-scheduler not support: <https://github.com/mvniekerk/tokio-cron-scheduler/issues/56>
    pub fn fix_delay_with_data<T: Serialize>(seconds: u64, data: T) -> JobBuilder<T> {
        JobBuilder {
            trigger: Trigger::FixedDelay(seconds),
            data: Some(data),
        }
    }
    pub fn fix_rate_with_data<T: Serialize>(seconds: u64, data: T) -> JobBuilder<T> {
        JobBuilder {
            trigger: Trigger::FixedRate(seconds),
            data: Some(data),
        }
    }
    pub fn cron_with_data<T: Serialize>(cron: &str, data: T) -> JobBuilder<T> {
        JobBuilder {
            trigger: Trigger::Cron(cron.to_string()),
            data: Some(data),
        }
    }
    pub fn build(self, app: Arc<App>) -> tokio_cron_scheduler::Job {
        let handler = self.handler;
        let mut job = match self.trigger {
            Trigger::OneShot(seconds) => tokio_cron_scheduler::Job::new_one_shot_async(
                Duration::from_secs(seconds),
                move |job_id, jobs| {
                    let handler = handler.clone();
                    let app = app.clone();
                    Box::pin(async move { handler.call(job_id, jobs, app).await })
                },
            ),
            // TODO
            Trigger::FixedDelay(seconds) => tokio_cron_scheduler::Job::new_repeated_async(
                Duration::from_secs(seconds),
                move |job_id, jobs| {
                    Box::pin(Self::call(handler.clone(), job_id, jobs, app.clone()))
                },
            ),
            Trigger::FixedRate(seconds) => tokio_cron_scheduler::Job::new_repeated_async(
                Duration::from_secs(seconds),
                move |job_id, jobs| {
                    Box::pin(Self::call(handler.clone(), job_id, jobs, app.clone()))
                },
            ),
            Trigger::Cron(schedule) => tokio_cron_scheduler::Job::new_async_tz(
                schedule.as_str(),
                chrono::Local,
                move |job_id, jobs| {
                    Box::pin(Self::call(handler.clone(), job_id, jobs, app.clone()))
                },
            ),
        }
        .expect("build job failed");
        if let Some(extra) = self.extra {
            let mut data = job.job_data().expect("get job_data failed");
            data.extra = extra;
            job.set_job_data(data).expect("set job_data failed");
        }
        job
    }

    async fn call(handler: BoxedHandler, job_id: JobId, jobs: JobScheduler, app: Arc<App>) {
        handler.call(job_id, jobs, app).await
    }
}

impl<T: Serialize> JobBuilder<T> {
    pub fn run<H, A>(self, handler: H) -> Job
    where
        H: Handler<A> + Sync,
        A: 'static,
    {
        Job {
            trigger: self.trigger,
            handler: BoxedHandler::from_handler(handler),
            extra: self
                .data
                .map(|data| serde_json::to_vec(&data).expect("job data to json failed")),
        }
    }
}
