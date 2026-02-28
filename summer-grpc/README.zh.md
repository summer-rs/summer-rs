[![crates.io](https://img.shields.io/crates/v/spring-grpc.svg)](https://crates.io/crates/spring-grpc)
[![Documentation](https://docs.rs/spring-grpc/badge.svg)](https://docs.rs/spring-grpc)

[tonic](https://github.com/hyperium/tonic) 是一个基于 Rust 的异步 gRPC 框架，用于构建高性能、类型安全的 gRPC 客户端和服务端。它建立在 tokio 和 hyper 之上，拥有良好的性能和生态集成，广泛应用于微服务通信、远程调用等场景。

## 依赖

```toml
spring-grpc = { version = "<version>" }
tonic = { version = "0.13" }
prost = { version = "0.13" }
```

## 配置项

```toml
[grpc]
binding = "172.20.10.4"                      # 要绑定的网卡IP地址，默认0.0.0.0
port = 8000                                  # 要绑定的端口号，默认9090
graceful = true                              # 是否开启优雅停机, 默认false
```

## 服务实现

基于protobuf协议定义接口

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

在`Cargo.toml`中添加protobuf编译依赖

```toml
[build-dependencies]
tonic-build = "0.13"
```

在`Cargo.toml`同级目录添加`build.rs`编译protobuf的接口定义生成对应的rust代码

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
```

实现相应的接口

```rust
use spring::plugin::service::Service;
use spring::App;
use spring_grpc::GrpcPlugin;
use tonic::{Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

/// 将protobuf定义的接口导入到项目中
pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() {
    App::new().add_plugin(GrpcPlugin).run().await
}

/// 派生Service，并指定Grpc Server
/// Grpc插件会自动将服务注册到tonic上
#[derive(Clone, Service)]
#[service(grpc = "GreeterServer")]
struct MyGreeter;

/// 实现protobuf定义的接口
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


完整代码参考[`grpc-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/grpc-example)
