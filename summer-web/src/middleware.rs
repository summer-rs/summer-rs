use crate::config::CorsMiddleware;
use crate::config::{
    EnableMiddleware, LimitPayloadMiddleware, Middlewares, StaticAssetsMiddleware,
    TimeoutRequestMiddleware, TraceLoggerMiddleware,
};
use crate::Router;
use anyhow::Context;
use axum::http::StatusCode;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use summer::error::Result;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::DefaultOnRequest;
use tower_http::trace::DefaultOnResponse;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use trace::DefaultOnEos;

pub use tower_http::*;

pub(crate) fn apply_middleware(mut router: Router, middleware: Middlewares) -> Router {
    // Always apply URI capture middleware first (for Problem Details)
    router = router.layer(axum::middleware::from_fn(
        crate::problem_details::capture_request_uri_middleware,
    ));

    if Some(EnableMiddleware { enable: true }) == middleware.catch_panic {
        router = router.layer(CatchPanicLayer::new());
    }
    if Some(EnableMiddleware { enable: true }) == middleware.compression {
        router = router.layer(CompressionLayer::new());
    }
    if let Some(TraceLoggerMiddleware { enable, level }) = middleware.logger {
        if enable {
            let level = level.into();
            router = router.layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().level(level))
                    .on_request(DefaultOnRequest::default().level(level))
                    .on_response(DefaultOnResponse::default().level(level))
                    .on_eos(DefaultOnEos::default().level(level)),
            );
        }
    }
    if let Some(TimeoutRequestMiddleware { enable, timeout }) = middleware.timeout_request {
        if enable {
            router = router.layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_millis(timeout),
            ));
        }
    }
    if let Some(LimitPayloadMiddleware { enable, body_limit }) = middleware.limit_payload {
        if enable {
            let limit = byte_unit::Byte::from_str(&body_limit)
                .unwrap_or_else(|_| panic!("parse limit payload str failed: {}", &body_limit));

            let limit = limit.as_u64() as usize;
            // Override axum's default 2MB body limit for extractors (Multipart, Json, etc.)
            router = router.layer(axum::extract::DefaultBodyLimit::max(limit));
            router = router.layer(RequestBodyLimitLayer::new(limit));
        }
    }
    if let Some(cors) = middleware.cors {
        if cors.enable {
            let cors = build_cors_middleware(&cors).expect("cors middleware build failed");
            router = router.layer(cors);
        }
    }
    if let Some(static_assets) = middleware.static_assets {
        if static_assets.enable {
            router = apply_static_dir(router, static_assets);
        }
    }
    router
}

fn apply_static_dir(router: Router, static_assets: StaticAssetsMiddleware) -> Router {
    if static_assets.must_exist
        && (!PathBuf::from(&static_assets.path).exists()
            || !PathBuf::from(&static_assets.fallback).exists())
    {
        panic!(
            "one of the static path are not found, Folder `{}` fallback: `{}`",
            static_assets.path, static_assets.fallback
        );
    }

    let fallback = ServeFile::new(format!("{}/{}", static_assets.path, static_assets.fallback));
    let serve_dir = ServeDir::new(static_assets.path).not_found_service(fallback);

    let service = if static_assets.precompressed {
        tracing::info!("[Middleware] Enable precompressed static assets");
        serve_dir.precompressed_gzip()
    } else {
        serve_dir
    };

    if static_assets.uri == "/" {
        router.fallback_service(service)
    } else {
        router.nest_service(&static_assets.uri, service)
    }
}

fn build_cors_middleware(cors: &CorsMiddleware) -> Result<CorsLayer> {
    let mut layer = CorsLayer::new();

    if let Some(allow_origins) = &cors.allow_origins {
        if allow_origins.iter().any(|item| item == "*") {
            layer = layer.allow_origin(cors::Any);
        } else {
            let mut origins = Vec::with_capacity(allow_origins.len());
            for origin in allow_origins {
                let origin = origin
                    .parse()
                    .with_context(|| format!("cors origin parse failed:{origin}"))?;
                origins.push(origin);
            }
            layer = layer.allow_origin(origins);
        }
    }

    if let Some(allow_headers) = &cors.allow_headers {
        if allow_headers.iter().any(|item| item == "*") {
            layer = layer.allow_headers(cors::Any);
        } else {
            let mut headers = Vec::with_capacity(allow_headers.len());
            for header in allow_headers {
                let header = header
                    .parse()
                    .with_context(|| format!("http header parse failed:{header}"))?;
                headers.push(header);
            }
            layer = layer.allow_headers(headers);
        }
    }

    if let Some(allow_methods) = &cors.allow_methods {
        if allow_methods.iter().any(|item| item == "*") {
            layer = layer.allow_methods(cors::Any);
        } else {
            let mut methods = Vec::with_capacity(allow_methods.len());
            for method in allow_methods {
                let method = method
                    .parse()
                    .with_context(|| format!("http method parse failed:{method}"))?;
                methods.push(method);
            }
            layer = layer.allow_methods(methods);
        }
    }

    if let Some(max_age) = cors.max_age {
        layer = layer.max_age(Duration::from_secs(max_age));
    }

    Ok(layer)
}

#[cfg(test)]
mod tests {
    use crate::WebConfigurator;
    use axum::body::Bytes;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use summer::plugin::ComponentRegistry;
    use summer::App;
    use tower::ServiceExt;

    async fn upload_handler(body: Bytes) -> String {
        format!("Received {} bytes", body.len())
    }

    /// After fix: TOML config `body_limit = "5MB"` allows 3MB upload.
    #[tokio::test]
    async fn test_configured_body_limit_allows_large_upload() {
        let toml_config = r#"
            [web.openapi]
            [web.middlewares]
            limit_payload = { enable = true, body_limit = "5MB" }
        "#;

        let built_app = App::new()
            .use_config_str(toml_config)
            .add_router(crate::Router::new().route("/upload", post(upload_handler)))
            .add_plugin(crate::WebPlugin)
            .build()
            .await
            .expect("Failed to build app");

        let router = built_app.get_component::<crate::Router>().unwrap();

        // 3MB — exceeds axum's default 2MB, within configured 5MB
        let body = "x".repeat(3 * 1024 * 1024);
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/upload")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Body exceeding configured limit is rejected.
    #[tokio::test]
    async fn test_configured_body_limit_rejects_oversized() {
        let toml_config = r#"
            [web.openapi]
            [web.middlewares]
            limit_payload = { enable = true, body_limit = "1KB" }
        "#;

        let built_app = App::new()
            .use_config_str(toml_config)
            .add_router(crate::Router::new().route("/upload", post(upload_handler)))
            .add_plugin(crate::WebPlugin)
            .build()
            .await
            .expect("Failed to build app");

        let router = built_app.get_component::<crate::Router>().unwrap();

        // 2KB — exceeds configured 1KB
        let body = "x".repeat(2 * 1024);
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/upload")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    /// Body within configured limit succeeds.
    #[tokio::test]
    async fn test_configured_body_limit_allows_within_limit() {
        let toml_config = r#"
            [web.openapi]
            [web.middlewares]
            limit_payload = { enable = true, body_limit = "1KB" }
        "#;

        let built_app = App::new()
            .use_config_str(toml_config)
            .add_router(crate::Router::new().route("/upload", post(upload_handler)))
            .add_plugin(crate::WebPlugin)
            .build()
            .await
            .expect("Failed to build app");

        let router = built_app.get_component::<crate::Router>().unwrap();

        let body = "x".repeat(512);
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/upload")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
