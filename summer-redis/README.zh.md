[![crates.io](https://img.shields.io/crates/v/spring-redis.svg)](https://crates.io/crates/spring-redis)
[![Documentation](https://docs.rs/spring-redis/badge.svg)](https://docs.rs/spring-redis)

## 依赖

```toml
spring-redis = { version = "<version>" }
```

## 配置项

```toml
[redis]
uri = "redis://127.0.0.1/"        # redis 数据库地址

# 下面都是可选配置
connection_timeout = 10000        # 连接超时时间，单位毫秒
response_timeout = 1000           # 响应超时时间，单位毫秒
number_of_retries = 6             # 重试次数，间隔时间按指数增长
exponent_base = 2                 # 间隔时间指数基数，单位毫秒
factor = 100                      # 间隔时间增长因子，默认100倍增长
max_delay = 60000                 # 最大间隔时间
```

## 组件

配置完上述配置项后，插件会自动注册一个[`Redis`](https://docs.rs/spring-redis/latest/spring_redis/type.Redis.html)连接管理对象。该对象是[`redis::aio::ConnectionManager`](https://docs.rs/redis/latest/redis/aio/struct.ConnectionManager.html)的别名。

```rust
pub type Redis = redis::aio::ConnectionManager;
```

## 提取插件注册的Component

`RedisPlugin`插件为我们自动注册了一个连接管理对象，我们可以使用`Component`从AppState中提取这个连接池，[`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html)是一个axum的[extractor](https://docs.rs/axum/latest/axum/extract/index.html)。

```rust
async fn list_all_redis_key(Component(mut redis): Component<Redis>) -> Result<impl IntoResponse> {
    let keys: Vec<String> = redis.keys("*").await.context("redis request failed")?;
    Ok(Json(keys))
}
```

## `cache`宏

`spring-redis`提供了基于 Redis 的异步函数透明缓存。在async方法上添加[`cache`](https://docs.rs/spring-redis/latest/spring_redis/attr.cache.html)宏即可对函数结果进行缓存。

示例如下：

```rust
#[cache("redis-cache:{key}", expire = 60, condition = key.len() > 3)]
async fn cachable_func(key: &str) -> String {
    format!("cached value for key: {key}")
}
```

`cache`宏支持`expire`、`condition`、`unless`三个可选参数。具体可以参考[`cache`](https://docs.rs/spring-redis/latest/spring_redis/attr.cache.html)文档。

`cache`包装的函数需满足以下要求：
- 必须是 `async fn`
- 可以返回 `Result<T, E>` 或普通值 `T`
- 返回类型必须实现 `serde::Serialize` 和 `serde::Deserialize`，底层使用`serde_json`进行序列化

完整代码参考[`redis-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/redis-example)