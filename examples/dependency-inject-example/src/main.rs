use anyhow::Context;
use derive_more::derive::Deref;
use schemars::JsonSchema;
use serde::Deserialize;
use summer::{
    auto_config,
    config::{ConfigRef, Configurable},
    plugin::{component::ComponentRef, service::Service, MutableComponentRegistry},
    App,
};
use summer_sqlx::{
    sqlx::{self, Row},
    ConnectPool, SqlxPlugin,
};
use summer_web::get;
use summer_web::{
    axum::response::IntoResponse, error::Result, extractor::Component, WebConfigurator, WebPlugin,
};
use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

#[derive(Clone, Deref)]
struct PageView(Arc<AtomicI32>);

// Main function entry
#[auto_config(WebConfigurator)] // auto config web router
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin) // Add plug-in
        .add_plugin(WebPlugin)
        .add_component(PageView(Arc::new(AtomicI32::new(0))))
        .run()
        .await
}

#[derive(Clone, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "user"]
struct UserConfig {
    username: String,
    project: String,
    #[serde(default)]
    star_count: i32,
}

#[derive(Clone)]
struct OptionalComponent;

#[derive(Clone, Service)]
struct UserService {
    #[inject(component)]
    db: ConnectPool,
    #[inject(component)]
    optional_comp: Option<OptionalComponent>, // OptionalComponent does not exist, so it is none
    #[inject(config)]
    config: UserConfig,
    #[inject(func = Self::init_star_count(&config))]
    count: Arc<AtomicI32>,
}

#[derive(Clone, Service)]
#[service(prototype)] // default builder fn is `build`
struct UserProtoService {
    #[inject(component)]
    count: PageView,
    #[inject(component)]
    optional_comp: Option<OptionalComponent>, // OptionalComponent does not exist, so it is none
    step: i32,
}

#[derive(Service)]
#[service(prototype = "build")]
struct UserProtoServiceWithLifetime<'s> {
    #[inject(component)]
    count: PageView,
    step: &'s i32,
}

impl UserService {
    pub async fn query_db(&self) -> Result<String> {
        let UserConfig {
            username, project, ..
        } = &self.config;

        let version: String = sqlx::query("select version() as version")
            .fetch_one(&self.db)
            .await
            .context("sqlx query failed")?
            .get("version");

        let pv_count = self.count.fetch_add(1, Ordering::SeqCst);
        Ok(format!(
            r#"
            The database used by {username}'s {project} is {version}.
            Page view counter is {pv_count}
            "#
        ))
    }

    fn init_star_count(config: &UserConfig) -> Arc<AtomicI32> {
        Arc::new(AtomicI32::new(config.star_count))
    }
}

/// For some large-sized components or configs, using Ref can avoid the performance impact of deep copying.
#[derive(Clone, Service)]
struct UserServiceUseRef {
    db: ComponentRef<ConnectPool>,
    config: ConfigRef<UserConfig>,
    #[inject(func = init_zero_count())]
    count: Arc<AtomicI32>,
}

fn init_zero_count() -> Arc<AtomicI32> {
    Arc::new(AtomicI32::new(0))
}

impl UserServiceUseRef {
    pub async fn query_db(&self) -> Result<String> {
        let UserConfig {
            username, project, ..
        } = &*self.config;

        let version: String = sqlx::query("select version() as version")
            .fetch_one(&*self.db)
            .await
            .context("sqlx query failed")?
            .get("version");

        let pv_count = self.count.fetch_add(1, Ordering::SeqCst);
        Ok(format!(
            r#"
            The database used by {username}'s {project} is {version}.
            Page view counter is {pv_count}
            "#
        ))
    }
}

impl UserProtoService {
    pub fn pv_count(&self) -> Result<String> {
        let Self { step, .. } = self;

        let pv_count = self.count.fetch_add(*step, Ordering::SeqCst);

        Ok(format!(
            r#"
            Page view counter is {pv_count}
            "#
        ))
    }
}

impl<'s> UserProtoServiceWithLifetime<'s> {
    pub fn pv_count(&self) -> Result<String> {
        let Self { step, .. } = self;

        let pv_count = self.count.fetch_add(**step, Ordering::SeqCst);

        Ok(format!(
            r#"
            Page view counter is {pv_count}
            "#
        ))
    }
}

#[get("/")]
async fn hello(Component(user_service): Component<UserService>) -> Result<impl IntoResponse> {
    assert!(user_service.optional_comp.is_none());
    Ok(user_service.query_db().await?)
}

#[get("/use-ref")]
async fn hello_ref(
    Component(user_service): Component<UserServiceUseRef>,
) -> Result<impl IntoResponse> {
    Ok(user_service.query_db().await?)
}

#[get("/prototype-service")]
async fn prototype_service() -> Result<impl IntoResponse> {
    let service = UserProtoService::build(5).context("build service failed")?;
    assert!(service.optional_comp.is_none());
    Ok(service.pv_count()?)
}

#[get("/prototype-service-lifetime")]
async fn prototype_service_with_lifetime() -> Result<impl IntoResponse> {
    let service = UserProtoServiceWithLifetime::build(&10).context("build service failed")?;
    Ok(service.pv_count()?)
}
