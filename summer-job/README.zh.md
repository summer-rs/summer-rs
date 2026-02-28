[![crates.io](https://img.shields.io/crates/v/summer-job.svg)](https://crates.io/crates/summer-job)
[![Documentation](https://docs.rs/summer-job/badge.svg)](https://docs.rs/summer-job)

## 依赖

```toml
summer-job = { version = "<version>" }
```

## API接口

App实现了[JobConfigurator](https://docs.rs/summer-job/latest/summer_job/trait.JobConfigurator.html)特征，可以通过该特征配置调度任务：

```rust, linenos, hl_lines=6 11-18
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

你也可以使用`auto_config`宏来实现自动配置，这个过程宏会自动将被过程宏标记的调度任务注册进app中：

```diff
+#[auto_config(JobConfigurator)]
 #[tokio::main]
 async fn main() {
     App::new()
         .add_plugin(JobPlugin)
         .add_plugin(SqlxPlugin)
-        .add_jobs(jobs())
         .run()
         .await
}
```

## 提取插件注册的Component

上面的`SqlxPlugin`插件为我们自动注册了一个Sqlx连接池组件，我们可以使用`Component`从App中提取这个连接池。需要注意`summer-job`的[`Component`](https://docs.rs/summer-job/latest/summer_job/extractor/struct.Component.html)和`summer-web`的[`Component`](https://docs.rs/summer-web/latest/summer_web/extractor/struct.Component.html)虽然实现原理类似，但这两个extractor归属不同的crate下。

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

## 读取配置

你可以用[`Config`](https://docs.rs/summer-job/latest/summer_job/extractor/struct.Config.html)抽取toml中的配置。用法上和[`summer-web`](https://summer-rs.github.io/zh/docs/plugins/summer-web/#du-qu-pei-zhi)完全一致。

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

在你的配置文件中添加相应配置：

```toml
[custom]
a = 1
b = true
```
