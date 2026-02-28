+++
title = "summer-rs initial version released"
description = "After a month of precipitation, I wrote a microservice framework similar to spring-boot in rust. The following is an example of the simplest web application"
date = 2024-08-04T09:19:42+00:00
updated = 2024-08-04T09:19:42+00:00
draft = false
template = "blog/page.html"

[extra]
lead = "After a month of precipitation, I wrote a microservice framework similar to spring-boot in rust. The following is an example of the simplest web application"
+++

```rust
use summer::{route, get, App};
use summer_web::{
    extractor::Path, handler::TypeRouter, response::IntoResponse, 
    Router, WebConfigurator, WebPlugin,
};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/")]
async fn hello_world() -> impl IntoResponse {
    "hello world"
}

#[route("/hello/{name}", method = "GET", method = "POST")]
async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    format!("hello {name}")
}
```

`summer-rs` uses plugins to integrate several popular frameworks in the rust ecosystem and provides procedural macros to simplify development.

If you are interested in `summer-rs`, you can [click here](/docs/getting-started/quick-start/) to get started quickly.