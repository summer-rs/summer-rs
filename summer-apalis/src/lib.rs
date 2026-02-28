use apalis::prelude::Monitor;
use summer::{
    app::AppBuilder,
    async_trait,
    error::Result,
    plugin::{component::ComponentRef, ComponentRegistry, MutableComponentRegistry, Plugin},
    signal,
};

pub use apalis;
#[cfg(feature = "amqp")]
pub use apalis_amqp;
#[cfg(feature = "board")]
pub use apalis_board;
#[cfg(feature = "sql-mysql")]
pub use apalis_mysql;
#[cfg(feature = "sql-postgres")]
pub use apalis_postgres;
#[cfg(feature = "redis")]
pub use apalis_redis;
#[cfg(feature = "sql-sqlite")]
pub use apalis_sqlite;
#[cfg(any(feature = "sql-postgres", feature = "sql-sqlite", feature = "sql-mysql"))]
pub use summer_sqlx;

pub struct ApalisPlugin;

pub type WorkerRegister = fn(&mut AppBuilder, Monitor) -> Monitor;

#[async_trait]
impl Plugin for ApalisPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        if let Some(builders) = app.get_component::<Vec<WorkerRegister>>() {
            let mut monitor = Monitor::new();
            for build_fn in &builders {
                monitor = build_fn(app, monitor);
            }
            if !builders.is_empty() {
                app.add_scheduler(move |_app| Box::new(Self::schedule(monitor)));
            }
        }
    }

    #[cfg(feature = "redis")]
    fn dependencies(&self) -> Vec<&str> {
        vec![std::any::type_name::<summer_redis::RedisPlugin>()]
    }

    #[cfg(all(
        any(feature = "sql-postgres", feature = "sql-sqlite", feature = "sql-mysql"),
        not(feature = "redis")
    ))]
    fn dependencies(&self) -> Vec<&str> {
        vec![std::any::type_name::<summer_sqlx::SqlxPlugin>()]
    }
}

impl ApalisPlugin {
    async fn schedule(monitor: Monitor) -> Result<String> {
        let _ = monitor.run_with_signal(shutdown_signal()).await;
        Ok("apalis scheduled finished".to_string())
    }
}

async fn shutdown_signal() -> std::io::Result<()> {
    let _ = signal::shutdown_signal("apalis").await;
    Ok(())
}

pub trait ApalisConfigurator {
    fn add_worker(&mut self, worker_register: WorkerRegister) -> &mut Self;
}

impl ApalisConfigurator for AppBuilder {
    fn add_worker(&mut self, worker_register: WorkerRegister) -> &mut Self {
        if let Some(workers) = self.get_component_ref::<Vec<WorkerRegister>>() {
            unsafe {
                let raw_ptr = ComponentRef::into_raw(workers);
                let workers = &mut *(raw_ptr as *mut Vec<WorkerRegister>);
                workers.push(worker_register);
            }
            self
        } else {
            self.add_component(vec![worker_register])
        }
    }
}
