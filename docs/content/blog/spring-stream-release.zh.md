+++
title = "summer-stream插件发布了"
description = "summer-stream插件是实时流式处理消息的编程工具"
date = 2024-08-25T09:19:42+00:00
updated = 2024-08-25T09:19:42+00:00
draft = false
template = "blog/page.html"

[extra]
lead = "summer-stream插件是实时流式处理消息的编程工具，可以大大简化基于文件、redis stream、kafka的消息处理"
+++

下面是一个简单的生产者：

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

Producer用于向消息存储发送消息。summer-stream底层使用sea-streamer实现，它抽象了file、stdio、redis、kafka消息存储层，使开发者可以用统一的接口来发送和处理消息。

下面是一个简单的消费者的代码：

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

[点击这里](/zh/docs/plugins/summer-stream/)可以查看相关文档。