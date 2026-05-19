# summer-nacos

Connect [summer-rs](https://github.com/summer-rs/summer-rs) applications to [Nacos](https://nacos.io) or [r-nacos](https://github.com/nacos-group/r-nacos) for configuration and service discovery.

Uses the official Rust client [`nacos-sdk`](https://crates.io/crates/nacos-sdk) (gRPC 2.x), which is compatible with r-nacos.

## Quick start

```toml
[dependencies]
summer-nacos = { path = "../summer-nacos" }
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

# Merged on ConfigEvent (order matters; later overrides earlier).
[[nacos.bootstrap]]
data_id = "app.toml"
group = "DEFAULT_GROUP"

[[nacos.bootstrap]]
data_id = "feature-flags.toml"
group = "DEFAULT_GROUP"

[nacos.registration]
service_name = "my-app"
# port is taken from summer-web when omitted
```

Run r-nacos locally:

```bash
docker run --name rnacos -p 8848:8848 -p 9848:9848 -p 10848:10848 -d qingpan/rnacos:stable
```

## Components

| Type | When |
|------|------|
| `NacosConfigService` | `enable_config = true` |
| `NacosNamingService` | `enable_naming = true` or `registration` is set |

With `summer-web` and/or `summer-grpc`, set `registration` to auto-register on [`ServerStartedEvent`](https://docs.rs/summer/latest/summer/event/struct.ServerStartedEvent.html) (with `protocol` metadata) and deregister on shutdown.

See [examples/integrations/nacos-example](../examples/integrations/nacos-example).
