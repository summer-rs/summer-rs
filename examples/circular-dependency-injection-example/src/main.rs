use schemars::JsonSchema;
use serde::Deserialize;
use summer::plugin::LazyComponent;
use summer::{auto_config, config::Configurable, plugin::service::Service, App};
use summer_web::get;
use summer_web::{
    axum::response::IntoResponse, error::Result, extractor::Component, WebConfigurator, WebPlugin,
};

// Main function entry
#[auto_config(WebConfigurator)] // auto config web router
#[tokio::main]
async fn main() {
    App::new().add_plugin(WebPlugin).run().await
}

#[derive(Clone, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "user"]
struct UserConfig {
    username: String,
    project: String,
    #[serde(default)]
    star_count: i32,
}

#[derive(Clone, Service)]
struct UserService {
    #[inject(config)]
    config: UserConfig,
    #[inject(component)]
    better_user: LazyComponent<BetterUserService>,
    // It's not necessary to use #[inject] for LazyComponent<T>
    // because it's automatically detected as lazy but you can still add it for keep
    // consistency.
    other_service: LazyComponent<OtherService>,
}

#[derive(Clone, Service)]
struct OtherService {
    #[inject(component)]
    user_service: LazyComponent<UserService>,
}

#[derive(Clone, Service)]
struct BetterUserService {
    #[inject(component)]
    user_service: UserService,
}

impl UserService {
    fn get_username(&self) -> String {
        let UserConfig {
            username,
            project,
            star_count,
        } = &self.config;
        format!("username: {username}, project: {project}, stars: {star_count}")
    }

    fn get_better_user_info(&self) -> summer::error::Result<String> {
        Ok(self
            .other_service
            .get()?
            .user_service
            .get()?
            .better_user
            .get()?
            .user_service
            .get_username())
    }
}

impl BetterUserService {
    fn get_info(&self) -> String {
        self.user_service
            .get_better_user_info()
            .unwrap_or_else(|_| "Failed to get better user info".to_string())
    }
}

#[get("/")]
async fn hello(Component(better): Component<BetterUserService>) -> Result<impl IntoResponse> {
    let info = better.get_info();
    Ok(info)
}
