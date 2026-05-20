# summer-nacos

将 [summer-rs](https://github.com/summer-rs/summer-rs) 应用接入 [Nacos](https://nacos.io) 或 [r-nacos](https://github.com/nacos-group/r-nacos)，用于配置中心与服务发现。

基于官方 Rust 客户端 [`nacos-sdk`](https://crates.io/crates/nacos-sdk)（gRPC 2.x），与 r-nacos 兼容。

## 快速开始

```toml
[dependencies]
summer-nacos = { version = "<version>" }
```

```rust
App::new()
    .add_plugin(NacosPlugin)
    .run()
    .await;
```

```toml
# config/app.toml
[nacos]
server_addr = "127.0.0.1:8848"
app_name = "my-app"
enable_config = true
enable_naming = true

# 在 ConfigEvent 时合并（顺序有意义，后项覆盖前项）。
[[nacos.bootstrap]]
data_id = "app.toml"
group = "DEFAULT_GROUP"

[[nacos.bootstrap]]
data_id = "feature-flags.toml"
group = "DEFAULT_GROUP"

[nacos.registration]
service_name = "my-app"
# 省略 port 时从 summer-web 读取
```

本地运行 r-nacos：

```bash
docker run --name rnacos -p 8848:8848 -p 9848:9848 -p 10848:10848 -d qingpan/rnacos:stable
```

## 组件

| 类型 | 启用条件 |
|------|----------|
| `NacosConfigService` | `enable_config = true` |
| `NacosNamingService` | `enable_naming = true` 或设置了 `registration` |

配合 `summer-web` 和/或 `summer-grpc` 时，配置 `registration` 可在 [`ServerStartedEvent`](https://docs.rs/summer/latest/summer/event/struct.ServerStartedEvent.html) 时自动注册（含 `protocol` 元数据），并在关闭时注销。

完整示例见 [examples/integrations/nacos-example](../examples/integrations/nacos-example)。
