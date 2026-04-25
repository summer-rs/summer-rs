[![crates.io](https://img.shields.io/crates/v/summer-pubsub.svg)](https://crates.io/crates/summer-pubsub)
[![Documentation](https://docs.rs/summer-pubsub/badge.svg)](https://docs.rs/summer-pubsub)

[google-cloud-pubsub](https://github.com/googleapis/google-cloud-rust) is a library for interacting with the Google Cloud Pub/Sub API.

## Dependencies

```toml
summer-pubsub = { version = "<version>" }
```

## Configuration items

```toml
[pubsub]
enabled = true # Whether to enable the pubsub plugin, default true
project_id = "your-project-id" # The project id of the google cloud project
credentials = "path/to/credentials.json" # The path to the credentials file
endpoint = "https://pubsub.googleapis.com" # The endpoint of the pubsub api in case you are using the pubsub emulator
```

The credentials file is a JSON file that contains the credentials to access the pubsub api. You can get the credentials file from the Google Cloud Console.

## Components

When the plugin is enabled, it will automatically register a `PubSubSubscriber` component.

`PubSubProducer` is a component that allows you to publish messages to the pubsub topic.

Besides you can create a handler for each subscription and register it to the app.

```rust
#[pubsub_listener("users_created_listener")]
async fn on_users_created_message(msg: Message) {
    println!("Received message: {:?}", msg);
}
```

On the handler, you can use `Component` to extract the components registered in the app or configs using `Config`.

```rust
use summer::extractor::{Component, Config};
use summer_pubsub::{PubSubConfig, pubsub_listener, Message};
use summer_sqlx::ConnectPool;

#[pubsub_listener("users_created_listener")]
async fn on_users_created_message(
    Config(_pubsub_config): Config<PubSubConfig>, // You can import other configs here if you need
    msg: Message,
    Component(_pool): Component<ConnectPool>,
) {
    println!("Received message: {:?}", msg);
}
```

The order of the parameters is not important in the handler, you can use the parameters in the handler as you like.

You can combine the `PubSubProducer` in web handler to publish messages to the pubsub topic.

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
