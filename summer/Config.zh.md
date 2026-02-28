你可以通过下面的方式定义配置：
```rust
#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "my-plugin"]
struct Config {
    a: u32,
    b: bool,
}
```

通过[`app.get_config()`](https://docs.rs/spring/latest/spring/app/struct.AppBuilder.html#method.get_config)方法可以读取`toml`中的配置：

```toml
[my-plugin]
a = 10
b = true
```

```rust
struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // 在自己的插件中加载配置
        let config = app.get_config::<Config>().expect("load config failed");
        //...
    }
}
```

## 在其他插件中使用配置

* [`spring-web`](https://spring-rs.github.io/zh/docs/plugins/spring-web/#du-qu-pei-zhi)
* [`spring-job`](https://spring-rs.github.io/zh/docs/plugins/spring-job/#du-qu-pei-zhi)
* [`spring-stream`](https://spring-rs.github.io/zh/docs/plugins/spring-stream/#du-qu-pei-zhi)

## 在配置文件中使用环境变量

spring-rs实现了一个简单的插值器。

可以在toml配置文件中使用`${ENV_VAR_NAME}`占位符读取环境变量的值。

如果值不存在则不替换占位符。

用`${ENV_VAR_NAME:default_value}`语法可以指定占位符的默认值。

```toml
[sea-orm]
uri = "${DATABASE_URL:postgres://postgres:xudjf23adj213@localhost/postgres}"
enable_logging = true
```

## 配置文件的自动补全提示

安装[vscode toml](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)插件，然后在`toml`配置文件第一行加上spring-rs的schema文件

```toml
#:schema https://spring-rs.github.io/config-schema.json
[web]
port = 18080
graceful = true
connect_info = true
```