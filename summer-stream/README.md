[![crates.io](https://img.shields.io/crates/v/spring-stream.svg)](https://crates.io/crates/spring-stream)
[![Documentation](https://docs.rs/spring-stream/badge.svg)](https://docs.rs/spring-stream)

## Dependencies

```toml
spring-stream = { version = "<version>", features=["file"] }
```

spring-stream supports four message storages: `file`, `stdio`, `redis`, and `kafka`.

optional features: `json`.

## Configuration items

```toml
[stream]
uri = "file://./stream"   # StreamerUri data stream address
```

StreamUri supports file, stdio, redis, and kafka. For the format of uri, please refer to [StreamerUri](https://docs.rs/sea-streamer/latest/sea_streamer/struct.StreamerUri.html).

* stdio is suitable for command line projects.
* file is suitable for stand-alone deployment projects.
* redis is suitable for distributed deployment projects. Redis5.0 provides stream data structure, so the redis version is required to be greater than 5.0. For details, please refer to [redis stream official documentation](https://redis.io/docs/latest/develop/data-types/streams/).
* Kafka is suitable for distributed deployment projects with larger message volumes. Kafka can be replaced with [redpanda](https://github.com/redpanda-data/redpanda), which is a high-performance message middleware written in C++ and compatible with the kafka protocol. It can completely get rid of the JVM that Kafka relies on.

### Detailed stream configuration
```toml
# File stream configuration
[stream.file]
connect = { create_file = "CreateIfNotExists" }

# Standard stream configuration
[stream.stdio]
connect = { loopback = false }

# Redis stream configuration
[stream.redis]
connect = { db=0,username="user",password="passwd" }

# Kafka stream configuration
[stream.kafka]
connect = { sasl_options={mechanism="Plain",username="user",password="passwd"}}
```

### Send message

`StreamPlugin` registers a `Producer` for sending messages. If you need to send messages in json format, you need to add the `json` feature in the dependencies:

```toml
spring-stream = { version = "0.1.1", features=["file","json"] }
```

```rust, linenos
{{ include_code(path="../../examples/stream-file-example/src/bin/producer.rs") }}
```

### Consume messages

`spring-stream` provides a process macro called `stream_listener` to subscribe to messages from a specified topic. The code is as follows:

```rust, linenos
{{ include_code(path="../../examples/stream-file-example/src/bin/consumer.rs") }}
``` 

View the complete example code [stream-file-example](https://github.com/spring-rs/spring-rs/tree/master/examples/stream-file-example), [stream-redis-example](https://github.com/spring-rs/spring-rs/tree/master/examples/stream-redis-example), [stream-kafka-example](https://github.com/spring-rs/spring-rs/tree/master/examples/stream-kafka-example)

## Read configuration

You can use [`Config`](https://docs.rs/spring-stream/latest/spring_stream/extractor/struct.Config.html) to extract the configuration in toml. The usage is exactly the same as [`spring-web`](https://spring-rs.github.io/zh/docs/plugins/spring-web/#du-qu-pei-zhi).

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

Add the corresponding configuration to your configuration file:

```toml
[custom]
a = 1
b = true
```