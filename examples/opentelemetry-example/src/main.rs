use anyhow::Context;
use summer::{
    tracing::{info, info_span, Instrument, Level},
    App,
};
use summer_opentelemetry::trace as otel_trace;
use summer_opentelemetry::{global, metrics as otel_metrics};
use summer_opentelemetry::{
    KeyValue, OpenTelemetryPlugin, ResourceConfigurator, SERVICE_NAME, SERVICE_VERSION,
};
use summer_sqlx::{
    sqlx::{self, Row},
    ConnectPool, SqlxPlugin,
};
use summer_web::{
    axum::response::IntoResponse,
    error::Result,
    extractor::{Component, Path},
    middleware::trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnRequest, DefaultOnResponse, TraceLayer,
    },
    Router, WebConfigurator, WebPlugin,
};
use summer_web::{get, route};

// Main function entry
#[tokio::main]
async fn main() {
    App::new()
        .opentelemetry_attrs([
            KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .add_router(router())
        .add_plugin(SqlxPlugin) // Add plug-in
        .add_plugin(WebPlugin)
        .add_plugin(OpenTelemetryPlugin)
        .run()
        .await
}

fn router() -> Router {
    let meter = global::meter(env!("CARGO_PKG_NAME"));
    let otel_metrics_layer = otel_metrics::HttpLayer::server(&meter);
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::default())
        .on_request(DefaultOnRequest::default())
        .on_response(DefaultOnResponse::default())
        .on_eos(DefaultOnEos::default());
    let otel_trace_layer = otel_trace::HttpLayer::server(Level::INFO).export_trace_id(true);
    // Note: http_tracing_layer must be added after trace_layer, because axum defaults to adding it first and executing it later.
    summer_web::handler::auto_router()
        .layer(otel_metrics_layer)
        .layer(trace_layer)
        .layer(otel_trace_layer)
}

// The get macro specifies the Http Method and request path.
// summer-rs also provides other standard http method macros such as post, delete, patch, etc.
#[get("/")]
async fn hello_world() -> impl IntoResponse {
    info!("hello world called");
    "hello world"
}

// You can also use the route macro to specify the Http Method and request path.
// Path extracts parameters from the HTTP request path
#[route("/hello/{name}", method = "GET", method = "POST")]
async fn hello(Path(name): Path<String>) -> impl IntoResponse {
    info!("hello {name} called");
    format!("hello {name}")
}

// Component can extract the connection pool registered by the SqlxPlugin in AppState
#[get("/version")]
async fn sqlx_request_handler(Component(pool): Component<ConnectPool>) -> Result<String> {
    info!("query sqlx version called");
    let version = sqlx::query("select version() as version")
        .fetch_one(&pool)
        .instrument(info_span!("sqlx-query"))
        .await
        .context("sqlx query failed")?
        .get("version");
    Ok(version)
}
