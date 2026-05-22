use anyhow::Context;
use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use serde::Deserialize;
use summer::{
    auto_config,
    component,
    config::Configurable,
    extractor::Config,
    App,
};
use summer_web::{
    axum::response::IntoResponse,
    extractor::{Component, Path},
    get, WebConfigurator, WebPlugin,
};
use tonic::transport::Channel;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Clone, Configurable, Deserialize)]
#[config_prefix = "greeter"]
struct GreeterClientConfig {
    endpoint: String,
}

#[component]
async fn create_greeter_client(
    Config(config): Config<GreeterClientConfig>,
) -> Result<GreeterClient<Channel>, anyhow::Error> {
    GreeterClient::connect(config.endpoint)
        .await
        .context("failed to connect server, please start server first")
}

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new().add_plugin(WebPlugin).run().await
}

#[get("/")]
async fn hello_index(
    Component(mut client): Component<GreeterClient<Channel>>,
) -> impl IntoResponse {
    client
        .say_hello(tonic::Request::new(HelloRequest {
            name: "world".into(),
        }))
        .await
        .expect("failed to say hello")
        .into_inner()
        .message
}

#[get("/hello/{name}")]
async fn hello(
    Path(name): Path<String>,
    Component(mut client): Component<GreeterClient<Channel>>,
) -> impl IntoResponse {
    client
        .say_hello(tonic::Request::new(HelloRequest { name }))
        .await
        .expect("failed to say hello")
        .into_inner()
        .message
}
