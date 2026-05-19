use anyhow::Context;
use grpc_nacos_example::discovery::{self, DiscoveryConfig};
use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use summer::{
    component,
    extractor::{Component, Config},
    plugin::ComponentRegistry,
    App,
};
use summer_nacos::{NacosNamingService, NacosPlugin};
use tonic::transport::Channel;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[component]
async fn create_greeter_client(
    Component(naming): Component<NacosNamingService>,
    Config(config): Config<DiscoveryConfig>,
) -> Result<GreeterClient<Channel>, anyhow::Error> {
    let endpoints = discovery::grpc_endpoints(&naming, &config)
        .await
        .context("discover gRPC servers from Nacos")?;
    println!(
        "discovered {} endpoint(s), load balancing: {}",
        endpoints.len(),
        endpoints.join(", ")
    );
    let channel = discovery::connect_balanced(endpoints)
        .await
        .context("build balanced gRPC channel")?;
    Ok(GreeterClient::new(channel))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut builder = App::new();
    builder.use_config_file("./config/client.toml");
    builder.add_plugin(NacosPlugin);
    let app = builder.build().await?;

    let mut client = app.get_expect_component::<GreeterClient<Channel>>();
    for i in 0..5 {
        let reply = client
            .say_hello(tonic::Request::new(HelloRequest {
                name: format!("nacos-{i}"),
            }))
            .await
            .with_context(|| format!("SayHello rpc (attempt {i})"))?
            .into_inner();
        println!("{i}: {}", reply.message);
    }
    Ok(())
}
