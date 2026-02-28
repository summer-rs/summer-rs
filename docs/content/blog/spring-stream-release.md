+++
title = "summer-stream plugin released"
description = "The summer-stream plugin is a programming tool for real-time streaming message processing, which can greatly simplify message processing based on files, redis stream, and kafka."
date = 2024-08-25T09:19:42+00:00
updated = 2024-08-25T09:19:42+00:00
draft = false
template = "blog/page.html"

[extra]
lead = "The summer-stream plugin is a programming tool for real-time streaming message processing, which can greatly simplify message processing based on files, redis stream, and kafka."
+++

Here is a simple producer: 

```rust
#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(StreamPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/")]
async fn send_msg(Component(producer): Component<Producer>) -> Result<impl IntoResponse> {
    let now = SystemTime::now();
    let json = json!({
        "success": true,
        "msg": format!("This message was sent at {:?}", now),
    });
    let resp = producer
        .send_json("topic", json)
        .await
        .context("send msg failed")?;

    let seq = resp.sequence();
    Ok(Json(json!({"seq":seq})))
}
```

Producer is used to send messages to the message store. summer-stream is implemented using sea-streamer at the bottom layer, which abstracts file, stdio, redis, and kafka The message storage layer allows developers to send and process messages using a unified interface.

Here is a simple consumer code:

```rust
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(StreamPlugin)
        .add_consumer(consumers())
        .run()
        .await
}

fn consumers() -> Consumers {
    Consumers::new().typed_consumer(listen_topic_do_something)
}

#[stream_listener("topic")]
async fn listen_topic_do_something(Json(payload): Json<Payload>) {
    tracing::info!("{:#?}", payload);
    // do something
}
```

[Click here](/zh/docs/plugins/summer-stream/) to view the relevant documentation.