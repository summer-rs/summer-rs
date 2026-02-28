macros for spring.rs.

## spring.rs macros re-exports

spring-rs re-exports all macros for this crate, so you usually don't need to explicitly specify this dependency. You can view the re-exported macros at spring-rs.

## Web route macros

### Single Method Handler

There is a macro to set up a handler for each of the most common HTTP methods.

See docs for: [GET], [POST], [PATCH], [PUT], [DELETE], [HEAD], [OPTIONS], [TRACE]

```
# use spring_web::axum::response::IntoResponse;
# use spring_macros::get;
#[get("/test")]
async fn get_handler() -> impl IntoResponse {
    "hello world"
}
```

### Multiple Method Handlers
Similar to the single method handler macro but takes one or more arguments for the HTTP methods
it should respond to. See [macro@route] macro docs.

```
# use spring_web::axum::response::IntoResponse;
# use spring_macros::route;
#[route("/test", method = "GET", method = "HEAD")]
async fn get_and_head_handler() -> impl IntoResponse {
    "hello world"
}
```

### Multiple Path Handlers
Acts as a wrapper for multiple single method handler macros. It takes no arguments and
delegates those to the macros for the individual methods. See [macro@routes] macro docs.

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