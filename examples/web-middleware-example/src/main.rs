use anyhow::Context;
use summer::{auto_config, App};
use summer_sqlx::sqlx::Row;
use summer_sqlx::{sqlx, ConnectPool, SqlxPlugin};
use summer_web::error::KnownWebError;
use summer_web::get;
use summer_web::nest;
use summer_web::{
    axum::{
        body,
        middleware::{self, Next},
        response::{IntoResponse, Response},
    },
    error::Result,
    extractor::Component,
    extractor::Request,
    WebPlugin,
};
use summer_web::{middlewares, WebConfigurator};
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(SqlxPlugin)
        .run()
        .await
}

/// Example #1:

/// Example of using `middlewares` macro to apply middleware to all routes in a module.
/// This module includes a problem detail middleware that handles errors and logs them to the database.
/// It also includes a timeout layer to limit request processing time.
/// The `hello_world` route returns a simple greeting, while the `sql_version` route
/// queries the database for its version. The `error_request` route demonstrates error handling.
#[middlewares(
    middleware::from_fn(problem_middleware),
    TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(10))
)]
mod routes {
    use summer_web::axum::http::StatusCode;

    use super::*;

    #[get("/")]
    async fn hello_world() -> impl IntoResponse {
        "hello world"
    }

    #[get("/version")]
    async fn sql_version(Component(pool): Component<ConnectPool>) -> Result<String> {
        let version = sqlx::query("select version() as version")
            .fetch_one(&pool)
            .await
            .context("sqlx query failed")?
            .get("version");
        Ok(version)
    }

    #[get("/error")]
    async fn error_request() -> Result<String> {
        Err(KnownWebError::bad_request("request error"))?
    }
}

/// ProblemDetail: https://www.rfc-editor.org/rfc/rfc7807
async fn problem_middleware(
    Component(db): Component<ConnectPool>,
    request: Request,
    next: Next,
) -> Response {
    let uri = request.uri().path().to_string();
    let response = next.run(request).await;
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("server body read failed");
        let msg = String::from_utf8(bytes.to_vec()).expect("read body to string failed");

        // error log into db
        let _ = sqlx::query("insert into error_log (msg, created_at) values ($1, now())")
            .bind(&msg)
            .execute(&db)
            .await;

        problemdetails::new(status)
            .with_instance(uri)
            .with_title(status.canonical_reason().unwrap_or("error"))
            .with_detail(msg)
            .into_response()
    } else {
        response
    }
}

/// Example #2:
/// This example demonstrates how to use the `middlewares` macro to apply multiple middleware layers to a module.
/// It includes a logging middleware, an authentication middleware, and a timeout layer.
/// The `protected` route is protected by the authentication middleware, which checks for an `Authorization` header.
/// If the header is missing, it returns a 401 Unauthorized response.

#[middlewares(
    middleware::from_fn(logging_middleware),
    middleware::from_fn(auth_middleware),
    TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(10)),
    CorsLayer::permissive()
)]
mod protected_routes {
    use summer_web::axum::http::StatusCode;

    use super::*;

    #[get("/protected")]
    async fn protected() -> impl IntoResponse {
        "Protected endpoint!"
    }
}

async fn logging_middleware(request: Request, next: Next) -> Response {
    println!("🔍 [LOGGING] {} {}", request.method(), request.uri().path());
    let response = next.run(request).await;
    println!("✅ [LOGGING] Response completed");
    response
}

async fn auth_middleware(request: Request, next: Next) -> Response {
    println!(
        "🔐 [AUTH] Checking authentication for: {}",
        request.uri().path()
    );

    if request.headers().get("Authorization").is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized".into())
            .unwrap();
    }

    next.run(request).await
}

/// Example #3:
/// Middlewares can also be applied to specific routes within a module.
/// This example demonstrates how to use the `middlewares` macro to apply
/// middlewares to a specific route and apply the module's middlewares to the
/// method router too.

#[middlewares(
    middleware::from_fn(logging_middleware),
    middleware::from_fn(auth_middleware),
    TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(10)),
    CorsLayer::permissive()
)]
#[nest("/api")]
mod api {

    use summer_web::{axum::http::StatusCode, extractor::Path};

    use super::*;

    #[middlewares(middleware::from_fn(problem_middleware))]
    #[get("/hello")]
    #[get("/hello/")]
    #[get("/hello/{user}")]
    pub async fn hello(user: Option<Path<String>>) -> Result<String> {
        let Some(user) = user else {
            return Err(KnownWebError::bad_request("request error"))?;
        };

        Ok(format!("Hello, {}!", user.0))
    }

    #[get("/error")]
    async fn error_request() -> Result<String> {
        Err(KnownWebError::internal_server_error("error!"))?
    }
}

/// Example #4:
/// This example demonstrates how to use the `middlewares` macro to apply middleware to specific routes.
/// It includes a logging middleware and a second route with its own logging middleware.

#[middlewares(middleware::from_fn(logging_middleware))]
#[get("/another_route")]
async fn another_route() -> impl IntoResponse {
    "Another Route"
}

/// Example #5:
/// This route demonstrates a simple goodbye endpoint without any middleware.
/// It returns a static string "goodbye world" when accessed.

#[get("/goodbye")]
async fn goodbye_world() -> impl IntoResponse {
    "goodbye world"
}
