[![crates.io](https://img.shields.io/crates/v/summer-job.svg)](https://crates.io/crates/summer-job)
[![Documentation](https://docs.rs/summer-job/badge.svg)](https://docs.rs/summer-job)

## Dependencies

```toml
summer-job = { version = "<version>" }
```

## API interface

App implements the [JobConfigurator](https://docs.rs/summer-job/latest/summer_job/trait.JobConfigurator.html) feature, which can be used to configure the scheduling task:

```rust, linenos, hl_lines=10 15-22
use summer::App;
use summer_job::{cron, JobPlugin, JobConfigurator, Jobs};
use summer_sqlx::SqlxPlugin;

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(JobPlugin)
        .add_plugin(SqlxPlugin)
        .add_jobs(jobs())
        .run()
        .await
}

fn jobs() -> Jobs {
    Jobs::new().typed_job(cron_job)
}

#[cron("1/10 * * * * *")]
async fn cron_job() {
    println!("cron scheduled: {:?}", SystemTime::now())
}
```

You can also use the `auto_config` macro to implement automatic configuration. This process macro will automatically register the scheduled tasks marked by the Procedural Macro into the app:

```diff
+#[auto_config(JobConfigurator)]
 #[tokio::main]
 async fn main() {
    App::new()
    .add_plugin(JobPlugin)
    .add_plugin(SqlxPlugin)
-   .add_jobs(jobs())
    .run()
    .await
}
```

## Extract the Component registered by the plugin

The `SqlxPlugin` plugin above automatically registers a Sqlx connection pool component for us. We can use `Component` to extract this connection pool from App. It should be noted that although the implementation principles of `summer-job`'s [`Component`](https://docs.rs/summer-job/latest/summer_job/extractor/struct.Component.html) and `summer-web`'s [`Component`](https://docs.rs/summer-web/latest/summer_web/extractor/struct.Component.html) are similar, these two extractors belong to different crates.

```rust
use summer_sqlx::{
    sqlx::{self, Row}, ConnectPool
};
use summer_job::cron;
use summer_job::extractor::Component;

#[cron("1/10 * * * * *")]
async fn cron_job(Component(db): Component<ConnectPool>) {
    let time: String = sqlx::query("select DATE_FORMAT(now(),'%Y-%m-%d %H:%i:%s') as time")
        .fetch_one(&db)
        .await
        .context("query failed")
        .unwrap()
        .get("time");
    println!("cron scheduled: {:?}", time)
}
```

## Read configuration

You can use [`Config`](https://docs.rs/summer-job/latest/summer_job/extractor/struct.Config.html) to extract the configuration in toml. The usage is exactly the same as [`summer-web`](https://summer-rs.github.io/zh/docs/plugins/summer-web/#du-qu-pei-zhi).


```rust
#[derive(Debug, Configurable, Deserialize)]
#[config_prefix = "custom"]
struct CustomConfig {
    a: u32,
    b: bool,
}

#[cron("1/10 * * * * *")]
async fn use_toml_config(Config(conf): Config<CustomConfig>) -> impl IntoResponse {
    format!("a={}, b={}", conf.a, conf.b)
}
```

Add the corresponding configuration to your configuration file:

```toml
[custom]
a = 1
b = true
```
