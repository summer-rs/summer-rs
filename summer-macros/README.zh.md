macros for spring.rs.

## spring-rs重新导出的宏

spring-rs重新导出了这个crate所有宏，因此你通常不需要明确的指定这个依赖。你可以到[spring-rs文档](https://docs.rs/spring/latest/spring/#attributes)查看被重新导出的宏。

## 绑定`spring-web`路由的宏

### 绑定单个Http Method的handler

有一类宏可以为常见的HTTP Method设置一个handler。

可以看这些相关文档: [GET], [POST], [PATCH], [PUT], [DELETE], [HEAD], [OPTIONS], [TRACE]

```
# use spring_web::axum::response::IntoResponse;
# use spring_macros::get;
#[get("/test")]
async fn get_handler() -> impl IntoResponse {
    "hello world"
}
```

### 绑定多个Http Method的handler

类似于单Method的handler宏，但它需要一个或多个method参数来指定 HTTP Method。请参阅 [macro@route] 宏文档。

```
# use spring_web::axum::response::IntoResponse;
# use spring_macros::route;
#[route("/test", method = "GET", method = "HEAD")]
async fn get_and_head_handler() -> impl IntoResponse {
    "hello world"
}
```

### 一个handler绑定多个路径

`routers`充当多个单Method宏的包装器。它不接受任何参数，它将这些参数委托给各个Method的宏。请参阅 [macro@routes] 宏文档。

```
# use spring_web::axum::response::IntoResponse;
# use spring_macros::routes;
#[routes]
#[get("/test")]
#[get("/test2")]
#[delete("/test")]
async fn example() -> impl IntoResponse {
    "hello world"
}
```

### 内嵌路由

`#[nest]`专门用于module上，可以针对一个模块下所有的handler的路由添加前缀。


```
# use spring_macros::{nest, get};
# use spring_web::axum::response::IntoResponse;
#[nest("/api")]
mod api {
    # use super::*;
    #[get("/hello")]
    pub async fn hello() -> impl IntoResponse {
        // this has path /api/hello
        "Hello, world!"
    }
}
```

## 绑定`spring-job`调度任务的宏

* `one_shot`：只调度一次
* `fix_delay`：按固定间隔时间调度任务，任务之间不会重叠，下一次调度需要上一次调度任务完成后等待固定间隔才开始
* `fix_rate`：按固定频率调度任务，如果任务比较耗时，任务之间可能会重叠。
* `cron`：按cron表达式指定的时间调度任务

