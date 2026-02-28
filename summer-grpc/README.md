[![crates.io](https://img.shields.io/crates/v/spring-grpc.svg)](https://crates.io/crates/spring-grpc)
[![Documentation](https://docs.rs/spring-grpc/badge.svg)](https://docs.rs/spring-grpc)

[tonic](https://github.com/hyperium/tonic) is a Rust-based asynchronous gRPC framework for building high-performance, type-safe gRPC clients and servers. It is built on tokio and hyper, has good performance and ecological integration, and is widely used in microservice communication, remote calls and other scenarios.

## Dependencies

```toml
spring-grpc = { version = "<version>" }
tonic = { version = "0.13" }
prost = { version = "0.13" }
```

## Configuration items

```toml
[grpc]
binding = "172.20.10.4"      # IP address of the network interface to be bound, default 0.0.0.0
port = 8000                  # Port number to be bound, default 9090
graceful = true              # Whether to enable graceful shutdown, default false
```

## Service implementation

Interface definition based on protobuf protocol

```proto
syntax = "proto3";

package helloworld;

// The greeting service definition.
service Greeter {
    // Sends a greeting
    rpc SayHello (HelloRequest) returns (HelloReply) {}
}

// The request message containing the user's name.
message HelloRequest {
    string name = 1;
}

// The response message containing the greetings
message HelloReply {
    string message = 1;
}
```

Add protobuf compilation dependency in `Cargo.toml`

```toml
[build-dependencies]
tonic-build = "0.13"
```

Add `build.rs` in the same directory as `Cargo.toml` to compile the interface definition of protobuf and generate the corresponding rust code

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
```

Implement the corresponding interface

```rust
use spring::plugin::service::Service;
use spring::App;
use spring_grpc::GrpcPlugin;
use tonic::{Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

/// Import the interface defined by protobuf into the project
pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() {
    App::new().add_plugin(GrpcPlugin).run().await
}

/// Derive Service and specify Grpc Server.
/// The GrpcPlugin will automatically register the service on tonic
#[derive(Clone, Service)]
#[service(grpc = "GreeterServer")]
struct MyGreeter;

/// Implement the interface defined by protobuf
#[tonic::async_trait]
impl Greeter for MyGreeter { 
    async fn say_hello( 
        &self, 
        request: Request<HelloRequest>, 
    ) -> Result<Response<HelloReply>, Status> { 
        println!("Got a request from {:?}", request.remote_addr()); 

        let reply = hello_world::HelloReply { 
            message: format!("Hello {}!", request.into_inner().name), 
        }; 
        Ok(Response::new(reply)) 
    }
}
```


Complete code reference [`grpc-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/grpc-example)