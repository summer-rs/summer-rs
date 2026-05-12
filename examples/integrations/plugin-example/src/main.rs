use schemars::JsonSchema;
use serde::Deserialize;
use summer::async_trait;
use summer::config::{ConfigRegistry, Configurable};
use summer::{
    app::{App, AppBuilder},
    plugin::Plugin,
};

struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        match app.get_config::<Config>() {
            Ok(config) => {
                println!("{:#?}", config);
                assert_eq!(config.a, 1);
                assert_eq!(config.b, true);
                assert_eq!(config.c.g, "hello");
                assert_eq!(config.d, "world");
                assert_eq!(config.e, ConfigEnum::EA);
                println!("c.f: {}", config.c.f);
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

#[derive(Debug, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "my-plugin"]
struct Config {
    a: u32,
    b: bool,
    c: ConfigInner,
    d: String,
    e: ConfigEnum,
}

#[derive(PartialEq, Debug, JsonSchema, Deserialize)]
enum ConfigEnum {
    EA,
    EB,
    EC,
    ED,
}

#[derive(Debug, JsonSchema, Deserialize)]
struct ConfigInner {
    f: u32,
    g: String,
}

#[tokio::main]
pub async fn main() {
    App::new().add_plugin(MyPlugin).run().await;
    println!("finish");
}
