[![crates.io](https://img.shields.io/crates/v/spring-job.svg)](https://crates.io/crates/spring-job)
[![Documentation](https://docs.rs/spring-job/badge.svg)](https://docs.rs/spring-job)

## Dependencies

```toml
spring-job = { version = "<version>" }
```

## API interface

App implements the [JobConfigurator](https://docs.rs/spring-job/latest/spring_job/trait.JobConfigurator.html) feature, which can be used to configure the scheduling task:

```rust, linenos, hl_lines=10 15-22
use spring::App;
use spring_job::{cron, JobPlugin, JobConfigurator, Jobs};
use spring_sqlx::SqlxPlugin;

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

The `SqlxPlugin` plugin above automatically registers a Sqlx connection pool component for us. We can use `Component` to extract this connection pool from App. It should be noted that although the implementation principles of `spring-job`'s [`Component`](https://docs.rs/spring-job/latest/spring_job/extractor/struct.Component.html) and `spring-web`'s [`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html) are similar, these two extractors belong to different crates.

```rust
use spring_sqlx::{
    sqlx::{self, Row}, ConnectPool
};
use spring_job::cron;
use spring_job::extractor::Component;

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

You can use [`Config`](https://docs.rs/spring-job/latest/spring_job/extractor/struct.Config.html) to extract the configuration in toml. The usage is exactly the same as [`spring-web`](https://spring-rs.github.io/zh/docs/plugins/spring-web/#du-qu-pei-zhi).


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
