[![crates.io](https://img.shields.io/crates/v/summer-test.svg)](https://crates.io/crates/summer-test)
[![Documentation](https://docs.rs/summer-test/badge.svg)](https://docs.rs/summer-test)

[summer-rs](https://github.com/summer-rs/summer-rs) 框架的测试工具库。

本 crate 基于 [`axum-test`](https://docs.rs/axum-test) 提供 Web 端到端（E2E）测试能力。在集成测试中，你可以走与生产环境相同的路由装配路径，而无需绑定 TCP 端口或进入服务循环。

## 依赖

在应用 crate 中作为 **dev-dependency** 引入：

```toml
[dev-dependencies]
summer-test = { version = "<version>" }
summer-web = { version = "<version>" }
tokio = { version = "1", features = ["full", "macros"] }
```

可选 **features**：

| Feature | 默认 | 说明 |
|---------|------|------|
| `web` | 是 | 启用 `MockWebPlugin`、`MockServer` 及 `axum_test` 重导出 |

## 快速开始

将 [`WebPlugin`](https://docs.rs/summer-web/latest/summer_web/struct.WebPlugin.html) 替换为 [`MockWebPlugin`](https://docs.rs/summer-test/latest/summer_test/struct.MockWebPlugin.html)，调用 [`build`](https://docs.rs/summer/latest/summer/app/struct.AppBuilder.html#method.build) 而非 [`run`](https://docs.rs/summer/latest/summer/app/struct.App.html#method.run)，然后通过内存中的测试服务器发起请求：

```rust
use summer::app::App;
use summer::plugin::ComponentRegistry;
use summer_test::{MockServer, MockWebPlugin};
use summer_web::{Router, WebConfigurator, axum::routing};

async fn ping() -> &'static str {
    "pong"
}

#[tokio::test]
async fn ping_returns_pong() -> summer::error::Result<()> {
    App::new()
        .add_plugin(MockWebPlugin)
        .add_router(Router::new().route("/ping", routing::get(ping)))
        .build()
        .await?
        .get_expect_component::<MockServer>()
        .get("/ping")
        .await
        .assert_status_ok()
        .assert_text("pong");

    Ok(())
}
```

## 工作原理

[`MockWebPlugin`](https://docs.rs/summer-test/latest/summer_test/struct.MockWebPlugin.html) 是 [`WebPlugin`](https://docs.rs/summer-web/latest/summer_web/struct.WebPlugin.html) 的即插即用替代品：

1. **相同的路由路径** — 复用 [`summer_web::assemble_router`](https://docs.rs/summer-web/latest/summer_web/fn.assemble_router.html) 与 [`summer_web::finalize_router`](https://docs.rs/summer-web/latest/summer_web/fn.finalize_router.html)，handler、中间件、layer 与 OpenAPI 行为与运行时一致。
2. **内存服务器** — 将最终路由包装为 [`axum_test::TestServer`](https://docs.rs/axum-test/latest/axum_test/struct.TestServer.html)，不绑定真实端口。
3. **立即返回** — 不注册调度器；`App::new().build().await` 完成后即返回，不会进入 serve 循环。
4. **插件兼容** — `name()` 返回 `"summer_web::WebPlugin"`，因此通过 [`Plugin::dependencies`](https://docs.rs/summer/latest/summer/plugin/trait.Plugin.html#method.dependencies) 依赖生产 Web 插件的其他插件在测试中仍能正确解析。
5. **合成启动事件** — 发布 [`ServerStartedEvent`](https://docs.rs/summer/latest/summer/event/struct.ServerStartedEvent.html)（地址 `127.0.0.1:0`，协议 HTTP），使 [`summer-nacos`](../summer-nacos) 等监听器在测试中保持行为一致。

[`MockServer`](https://docs.rs/summer-test/latest/summer_test/struct.MockServer.html) 作为普通组件注册在构建后的 [`App`](https://docs.rs/summer/latest/summer/app/struct.App.html) 上，并实现 [`Deref<Target = TestServer>`](https://doc.rust-lang.org/std/ops/trait.Deref.html)，可直接链式调用 HTTP 方法：

```rust
app.get_expect_component::<MockServer>()
    .post("/echo")
    .text("hello")
    .await
    .assert_status_ok();
```

本 crate 还重导出 [`axum_test`](https://docs.rs/axum-test)，便于编写断言或自定义请求辅助函数。

## 与其他插件联测

`MockWebPlugin` 可与生产环境中使用的任意插件组合。通过 [`.use_config_str`](https://docs.rs/summer/latest/summer/app/struct.AppBuilder.html#method.use_config_str) 或测试配置文件注入配置：

```rust
App::new()
    .use_config_str(r#"
[postgres]
connect = "postgres://postgres:postgres@127.0.0.1:5432/postgres"
"#)
    .add_plugin(PgPlugin)
    .add_plugin(MockWebPlugin)
    .add_router(router())
    .build()
    .await?;
```

涉及数据库的测试可配合 [testcontainers](https://crates.io/crates/testcontainers)：启动容器、通过 `.use_config_str` 注入连接串，再对 HTTP 响应做断言。Postgres 与 Redis 的完整示例见本 crate 的 [`web_e2e`](tests/web_e2e.rs) 测试。

> **NOTE**：基于容器的测试需要可用的 Docker 守护进程。

## API 概览

| 项 | 说明 |
|----|------|
| [`MockWebPlugin`](https://docs.rs/summer-test/latest/summer_test/struct.MockWebPlugin.html) | 测试用 `WebPlugin` 替代品 |
| [`MockServer`](https://docs.rs/summer-test/latest/summer_test/struct.MockServer.html) | 作为 `App` 组件存储的内存 `TestServer` 句柄 |
| [`axum_test`](https://docs.rs/axum-test) | 重导出，提供请求/响应辅助与断言 |

## 运行本 crate 的测试

在 workspace 根目录执行：

```bash
cargo test -p summer-test
```

Postgres 与 Redis 的 E2E 测试（`mock_web_plugin_with_postgres_container`、`mock_web_plugin_with_redis_container`）需要 Docker 环境。
