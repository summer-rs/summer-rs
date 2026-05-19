use summer::plugin::service::Service;
use summer::App;
use summer_grpc::GrpcPlugin;
use summer_nacos::NacosPlugin;
use tonic::{Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(NacosPlugin)
        .add_plugin(GrpcPlugin)
        .run()
        .await
}

#[derive(Clone, Service)]
#[service(grpc = "GreeterServer")]
struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("SayHello from {:?}", request.remote_addr());
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}
