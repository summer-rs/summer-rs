[![crates.io](https://img.shields.io/crates/v/summer-pubsub.svg)](https://crates.io/crates/summer-pubsub)
[![Documentation](https://docs.rs/summer-pubsub/badge.svg)](https://docs.rs/summer-pubsub)

[google-cloud-pubsub](https://github.com/googleapis/google-cloud-rust) 是一个用于与 Google Cloud Pub/Sub API 交互的库。

## 依赖

```toml
summer-pubsub = { version = "<version>" }
```

## 配置项

```toml
[pubsub]
enabled = true # 是否启用 pubsub 插件，默认 true
project_id = "your-project-id" # Google Cloud 项目的 project id
credentials = "path/to/credentials.json" # 凭证文件路径
endpoint = "https://pubsub.googleapis.com" # 如果使用 pubsub 模拟器，可指定 API endpoint
```

凭证文件是一个 JSON 文件，包含访问 pubsub API 所需的认证信息。你可以从 Google Cloud Console 获取该文件。

## 组件

当插件启用时，会自动注册一个 `PubSubSubscriber` 组件。

`PubSubProducer` 是一个用于向 pubsub topic 发布消息的组件。

此外，你可以为每个订阅创建一个 handler，并将其注册到应用中。

```rust
#[pubsub_listener("users_created_listener")]
async fn on_users_created_message(msg: Message) {
    println!("Received message: {:?}", msg);
}
```

在 handler 中，你可以使用 `Component` 提取应用中注册的组件，或者使用 `Config` 获取配置。

```rust
use summer::extractor::{Component, Config};
use summer_pubsub::{PubSubConfig, pubsub_listener, Message};
use summer_sqlx::ConnectPool;

#[pubsub_listener("users_created_listener")]
async fn on_users_created_message(
    Config(_pubsub_config): Config<PubSubConfig>, // 如果需要，可以在这里引入其他配置
    msg: Message,
    Component(_pool): Component<ConnectPool>,
) {
    println!("Received message: {:?}", msg);
}
```

handler 中参数的顺序并不重要，你可以根据需要自由排列。

你也可以在 Web handler 中结合使用 `PubSubProducer`，向 pubsub topic 发布消息。

```rust
use summer_web::axum::response::IntoResponse;
use summer_web::extractor::Component;
use summer_web::get;
use summer_pubsub::{PubSubProducer};
use summer_pubsub::model::Message;

#[get("/pubsub")]
async fn pubsub(
    Component(producer): Component<PubSubProducer>,
) -> impl IntoResponse {
    
    let message = Message::new()
        .set_data("Hello, world!".as_bytes());
    
    producer.publish("users_created", message).await;

    (StatusCode::OK, "Message published").into_response()
}
```
