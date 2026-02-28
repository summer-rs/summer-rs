[![crates.io](https://img.shields.io/crates/v/spring-redis.svg)](https://crates.io/crates/spring-redis)
[![Documentation](https://docs.rs/spring-redis/badge.svg)](https://docs.rs/spring-redis)

## Dependencies

```toml
spring-redis = { version = "<version>" }
```

## Configuration items

```toml
[redis]
uri = "redis://127.0.0.1/" # redis database address

# The following are all optional configurations
connection_timeout = 10000  # Connection timeout, in milliseconds
response_timeout = 1000     # Response timeout, in milliseconds
number_of_retries = 6       # Retry times, interval time increases exponentially
exponent_base = 2           # Interval time exponential base, unit milliseconds
factor = 100                # Interval time growth factor, default 100 times growth
max_delay = 60000           # Maximum interval time
```

## Component

After configuring the above configuration items, the plugin will automatically register a [`Redis`](https://docs.rs/spring-redis/latest/spring_redis/type.Redis.html) connection management object. This object is an alias of [`redis::aio::ConnectionManager`](https://docs.rs/redis/latest/redis/aio/struct.ConnectionManager.html).

```rust
pub type Redis = redis::aio::ConnectionManager;
```

## Extract the Component registered by the plugin

The `RedisPlugin` plugin automatically registers a connection management object for us. We can use `Component` to extract this connection pool from AppState. [`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html) is an axum [extractor](https://docs.rs/axum/latest/axum/extract/index.html).

```rust
async fn list_all_redis_key(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let keys: Vec<String> = redis.keys("*").await.context("redis request failed")?;
    Ok(Json(keys))
}
```

## `cache` macro

`spring-redis` provides a transparent cache for asynchronous functions based on Redis. Add the [`cache`](https://docs.rs/spring-redis/latest/spring_redis/attr.cache.html) macro to the async method to cache the function result.

The example is as follows:

```rust
#[cache("redis-cache:{key}", expire = 60, condition = key.len() > 3)]
async fn cachable_func(key: &str) -> String {
    format!("cached value for key: {key}")
}
```

The `cache` macro supports three optional parameters: `expire`, `condition`, and `unless`. For details, please refer to the [`cache`](https://docs.rs/spring-redis/latest/spring_redis/attr.cache.html) document.

The function wrapped by `cache` must meet the following requirements:

- Must be `async fn`
- Can return `Result<T, E>` or a normal value `T`
- The return type must implement `serde::Serialize` and `serde::Deserialize`, and the underlying `serde_json` is used for serialization

Complete code reference [`redis-example`][redis-example]

[redis-example]: https://github.com/spring-rs/spring-rs/tree/master/examples/redis-example