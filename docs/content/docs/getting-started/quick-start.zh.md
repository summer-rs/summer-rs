+++
title = "快速上手"
description = "一个页面介绍如何快速上手summer-rs"
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "在这个页面，我会介绍如何快速上手summer-rs"
toc = true
top = false
+++

## 准备环境

* rust ≥ 1.75

## 添加依赖

在你的`Cargo.toml`文件中添加下面的依赖

```toml
[dependencies]
# summer提供了核心的插件系统和有用的过程宏
summer = "<version>"
# 如果你准备写web应用就添加summer-web
summer-web = "<version>"
# 如果应用需要和数据库交互就添加summer-sqlx
summer-sqlx = { version="<version>", features = ["mysql"] }
# summer-rs项目默认使用tokio异步运行时
tokio = "1"
```

## 编写代码

```rust
use anyhow::Context;
use summer::{auto_config, App};
use summer_sqlx::{
    sqlx::{self, Row},
    ConnectPool, SqlxPlugin,
};
use summer_web::{
    axum::response::IntoResponse,
    error::Result,
    extractor::{Component, Path},
    WebConfigurator, WebPlugin,
};
use summer_web::{get, route};

// 主函数入口
#[auto_config(WebConfigurator)]   // 自动扫描web router
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin)  // 添加插件
        .add_plugin(WebPlugin)
        .run()
        .await
}

// get宏指定Http Method和请求路径。summer-rs还提供了post、delete、patch等其他标准http method宏
#[get("/")]
async fn hello_world() -> impl IntoResponse {
    "hello world"
}

// 也可以使用route宏指定Http Method和请求路径。Path从HTTP请求中提取请求路径中的参数
#[route("/hello/{name}", method = "GET", method = "POST")]
async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    format!("hello {name}")
}

// Component可以抽取由Sqlx插件在AppState中的注册的连接池
#[get("/version")]
async fn sqlx_request_handler(Component(pool): Component<ConnectPool>) -> Result<String> {
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
```

## 对应用进行配置

在项目的根路径下创建一个`config`目录，这里会存储`summer-rs`的配置文件。

你可以在该目录下先创建一个`app.toml`文件，内容如下：

```toml
[web]
port = 8000                  # 配置web服务端口，如果不配置默认就是8080端口

[sqlx]                       # 配置sqlx的数据库连接信息
uri = "mysql://user:password@127.0.0.1:3306"
```

`summer-rs`支持多环境配置：dev(开发)、test(测试)、prod(生产)，分别对应着`app-dev.toml`、`app-test.toml`、`app-prod.toml`三个配置文件。环境配置文件中的配置会覆盖`app.toml`主配置文件的配置项。

`summer-rs`会根据`summer_ENV`环境变量激活对应环境的配置文件。

## 运行

编码完成，请确保你的数据库能正常连接，然后就让我们开始运行起来吧。

```sh
cargo run
```
