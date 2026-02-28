[![crates.io](https://img.shields.io/crates/v/spring.svg)](https://crates.io/crates/spring)
[![Documentation](https://docs.rs/spring/badge.svg)](https://docs.rs/spring)

## Introduction

`spring` is the core module of this project, which includes: configuration management, plugin management, and component management.

* All plugins need to implement the [`Plugin`](https://docs.rs/spring/latest/spring/plugin/trait.Plugin.html) trait.
* All configurations need to implement the [`Configurable`](https://docs.rs/spring/latest/spring/config/trait.Configurable.html) trait.
* All components need to implement the [`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html) trait.

> Note: To avoid deep copying of large struct in Component, it is recommended to use the [newtype pattern](https://effective-rust.com/newtype.html) to reference them via `Arc<T>`.

## How to write your own plugin

Add dependencies

```toml
spring = { version = "<version>" }           # This crate contains the definition of plugin traits
serde = { workspace = true, features = ["derive"] } # Used to parse plugin configuration items
```

```rust
use serde::Deserialize;
use spring::async_trait;
use spring::config::{Configurable, ConfigRegistry};
use spring::{app::AppBuilder, plugin::Plugin};

struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // Call app.get_config::<Config>() method to get configuration items
        match app.get_config::<Config>() {
            Ok(config) => {
                println!("{:#?}", config);
                assert_eq!(config.a, 1);
                assert_eq!(config.b, true);

                // Get the configuration items to build the corresponding components

            }
            Err(e) => println!("{:?}", e),
        }
    }
}

/// Plugin configuration
#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "my-plugin"]
struct Config {
    a: u32,
    b: bool,
}
```

For the complete code, refer to [`plugin-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/plugin-example), or refer to other built-in plugin codes.