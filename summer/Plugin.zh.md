[![crates.io](https://img.shields.io/crates/v/spring.svg)](https://crates.io/crates/spring)
[![Documentation](https://docs.rs/spring/badge.svg)](https://docs.rs/spring)

## 介绍

`spring`是该项目的核心模块，包含了：配置管理、插件管理、组件管理。

* 所有的插件都需要实现[`Plugin`](https://docs.rs/spring/latest/spring/plugin/trait.Plugin.html)特征。
* 所有的配置都需要实现[`Configurable`](https://docs.rs/spring/latest/spring/config/trait.Configurable.html)特征。
* 所有的组件都需要实现[`Clone`](https://doc.rust-lang.org/std/clone/trait.Clone.html)特征。

> 注意：为了避免对Component内大结构体进行深拷贝，推荐使用[newtype模式](https://effective-rust.com/newtype.html)通过`Arc<T>`进行引用。

## 如何编写自己的插件

添加依赖

```toml
spring = { version = "<version>" }                       # 该crate中包含了插件trait的定义
serde = { workspace = true, features = ["derive"] }  # 用于解析插件的配置项
```

```rust
struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // 调用app.get_config::<Config>()方法即可获取配置项
        match app.get_config::<Config>() {
            Ok(config) => {
                println!("{:#?}", config);
                assert_eq!(config.a, 1);
                assert_eq!(config.b, true);

                // 拿到配置项即可构建相应的组件

            }
            Err(e) => println!("{:?}", e),
        }
    }
}

/// 插件的配置
#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "my-plugin"]
struct Config {
    a: u32,
    b: bool,
}
```

完整代码参考[`plugin-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/plugin-example)，也可以参考自带的其他插件代码。