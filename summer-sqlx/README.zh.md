[![crates.io](https://img.shields.io/crates/v/spring-sqlx.svg)](https://crates.io/crates/spring-sqlx)
[![Documentation](https://docs.rs/spring-sqlx/badge.svg)](https://docs.rs/spring-sqlx)

[SQLx](https://github.com/launchbadge/sqlx) 是一个异步 SQL 库，通过编译期校验 SQL 语句来在不使用 ORM 的情况下提供强类型、零运行时反射的数据库访问能力。

## 依赖

```toml
spring-sqlx = { version = "<version>", features = ["mysql"] }
```

可以替换`postgres`、`mysql`、`sqlite`feature来选择合适的数据库驱动。

可选的features: 
* `with-json`
* `with-chrono`
* `with-rust_decimal`
* `with-bigdecimal`
* `with-uuid`
* `with-time`

## 配置项

```toml
[sqlx]
uri = "postgres://root:123456@localhost:5432/pg_db"  # 数据库地址
min_connections = 1                                  # 连接池的最小连接数，默认值为1
max_connections = 10                                 # 连接池的最大连接数，默认值为10
acquire_timeout = 30000                              # 占用连接超时时间，单位毫秒，默认30s
idle_timeout = 600000                                # 连接空闲时间，单位毫秒，默认10min
connect_timeout = 1800000                            # 连接的最大存活时间，单位毫秒，默认30min
```

## 组件

配置完上述配置项后，插件会自动注册一个[`ConnectPool`](https://docs.rs/spring-sqlx/latest/spring_sqlx/type.ConnectPool.html)连接池对象。该对象是[`sqlx::AnyPool`](https://docs.rs/sqlx/latest/sqlx/type.AnyPool.html)的别名。

```rust
pub type ConnectPool = sqlx::AnyPool;
```

## 提取插件注册的Component

`SqlxPlugin`插件为我们自动注册了一个Sqlx连接池组件，我们可以使用`Component`从AppState中提取这个连接池，[`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html)是一个axum的[extractor](https://docs.rs/axum/latest/axum/extract/index.html)。

```rust
use spring_sqlx::{sqlx::{self, Row}, ConnectPool};
use spring_web::get;
use spring_web::extractor::Component;
use spring_web::error::Result;
use anyhow::Context;

#[get("/version")]
async fn mysql_version(Component(pool): Component<ConnectPool>) -> Result<String> {
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
```

完整代码参考[`sqlx-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/sqlx-example)