use schemars::JsonSchema;
use serde::Deserialize;
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
}

#[derive(Clone, Service)]
struct BetterUserService {
    #[inject(component)]
    user_service: UserService,
}

impl BetterUserService {
    fn get_info(&self) -> String {
        let UserConfig {
            username,
            project,
            star_count,
        } = &self.user_service.config;
        format!("username: {username}, project: {project}, stars: {star_count}")
    }
}

#[get("/")]
async fn hello(Component(better_user): Component<BetterUserService>) -> Result<impl IntoResponse> {
    Ok(better_user.get_info())
}
