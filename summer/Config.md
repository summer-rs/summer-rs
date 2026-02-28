You can define configuration in the following way:
```rust
use summer::config::Configurable;
use serde::Deserialize;

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "my-plugin"]
struct Config {
    a: u32,
    b: bool,
}
```

The configuration in `toml` can be read through the [`app.get_config()`](https://docs.rs/summer/latest/summer/app/struct.AppBuilder.html#method.get_config) method:

```toml
[my-plugin]
a = 10
b = true
```

```rust, hl_lines=19
use summer::async_trait;
use summer::plugin::Plugin;
use summer::config::Configurable;
use serde::Deserialize;

#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "my-plugin"]
struct Config {
    a: u32,
    b: bool,
}

struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // Loading configuration in your own plugin
        let config = app.get_config::<Config>().expect("load config failed");
        // do something...
    }
}
```

## Use configuration in other plugins

* [`summer-web`](https://summer-rs.github.io/docs/plugins/summer-web/#read-configuration)
* [`summer-job`](https://summer-rs.github.io/docs/plugins/summer-job/#read-configuration)
* [`summer-stream`](https://summer-rs.github.io/docs/plugins/summer-stream/#read-configuration)

## Using environment variables in configuration files

summer-rs implements a simple interpolator.

You can use the `${ENV_VAR_NAME}` placeholder in the toml configuration file to read the value of the environment variable.

If the value does not exist, the placeholder is not replaced.

You can specify the default value of the placeholder using the `${ENV_VAR_NAME:default_value}` syntax.

```toml
[sea-orm]
uri = "${DATABASE_URL:postgres://postgres:xudjf23adj213@localhost/postgres}"
enable_logging = true
```

## Auto-completion tips for the configuration file

Install the [vscode toml](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml) plugin, then add the summer-rs schema file to the first line of the `toml` configuration file.

```toml
#:schema https://summer-rs.github.io/config-schema.json
[web]
port = 18080
graceful = true
connect_info = true
```