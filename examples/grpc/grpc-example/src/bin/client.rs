use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use summer::{auto_config, plugin::MutableComponentRegistry, App};
use summer_web::{
    axum::response::IntoResponse,
    extractor::{Component, Path},
    get, WebConfigurator, WebPlugin,
};
use tonic::transport::Channel;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    let client = GreeterClient::connect("http://127.0.0.1:9090")
        .await
        .expect("failed to connect server, please start server first");
    App::new()
        .add_plugin(WebPlugin)
        .add_component(client)
        .run()
        .await
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
