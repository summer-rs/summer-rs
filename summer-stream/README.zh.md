[![crates.io](https://img.shields.io/crates/v/spring-stream.svg)](https://crates.io/crates/spring-stream)
[![Documentation](https://docs.rs/spring-stream/badge.svg)](https://docs.rs/spring-stream)

## 依赖

```toml
spring-stream = { version = "<version>",features=["file"] }
```

spring-stream支持`file`、`stdio`、`redis`、`kafka`四种消息存储。

可选的features: `json`.

## 配置项

```toml
[stream]
uri = "file://./stream"      # StreamerUri 数据流地址
```

StreamUri支持file、stdio、redis、kafka。uri的格式具体参考[StreamerUri](https://docs.rs/sea-streamer/latest/sea_streamer/struct.StreamerUri.html)。

* stdio适合命令行项目。
* file适合单机部署的项目。
* redis适合分布式部署的项目。Redis5.0提供了stream数据结构，因此要求redis版本大于5.0。详细可以参考[redis stream官方文档](https://redis.io/docs/latest/develop/data-types/streams/)。
* kafka适合消息量更大的分布式部署的项目。Kafka可以用兼容[redpanda](https://github.com/redpanda-data/redpanda)替代，它是C++编写的兼容kafka协议的高性能消息中间件，用它可以彻底摆脱Kafka依赖的JVM。

### 流的详细配置
```toml
# 文件流配置
[stream.file]
connect = { create_file = "CreateIfNotExists" }

# 标准流配置
[stream.stdio]
connect = { loopback = false }

# redis流配置
[stream.redis]
connect = { db=0,username="user",password="passwd" }

# kafka流配置
[stream.kafka]
connect = { sasl_options={mechanism="Plain",username="user",password="passwd"}}
```

### 发送消息

`StreamPlugin`注册了一个`Producer`用于发送消息。如果需要发送json格式的消息，需要在依赖项中添加`json`的feature：

```toml
spring-stream = { version = "0.1.1", features=["file","json"] }
```

```rust, linenos
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

### 消费消息

`spring-stream`提供了`stream_listener`的过程宏来订阅指定topic的消息，代码如下：

```rust, linenos, hl_lines=5 10-17
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

#[stream_listener("topic", file_consumer_options = fill_file_consumer_options)]
async fn listen_topic_do_something(Json(payload): Json<Payload>) {
    tracing::info!("{:#?}", payload);
}

fn fill_file_consumer_options(opts: &mut FileConsumerOptions) {
    opts.set_auto_stream_reset(AutoStreamReset::Earliest);
}
```

完整示例代码查看[stream-file-example](https://github.com/spring-rs/spring-rs/tree/master/examples/stream-file-example)、[stream-redis-example](https://github.com/spring-rs/spring-rs/tree/master/examples/stream-redis-example)、[stream-kafka-example](https://github.com/spring-rs/spring-rs/tree/master/examples/stream-kafka-example)

## 读取配置

你可以用[`Config`](https://docs.rs/spring-stream/latest/spring_stream/extractor/struct.Config.html)抽取toml中的配置。用法上和[`spring-web`](https://spring-rs.github.io/zh/docs/plugins/spring-web/#du-qu-pei-zhi)完全一致。

```rust
#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "custom"]
struct CustomConfig {
    a: u32,
    b: bool,
}

#[stream_listener("topic")]
async fn use_toml_config(Config(conf): Config<CustomConfig>) -> impl IntoResponse {
    format!("a={}, b={}", conf.a, conf.b)
}
```

在你的配置文件中添加相应配置：

```toml
[custom]
a = 1
b = true
```