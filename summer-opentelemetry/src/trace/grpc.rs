//! Middleware that adds tracing to a [`Service`] that handles gRPC requests.
//! refs: https://opentelemetry.io/docs/specs/semconv/rpc/grpc/

use super::SpanKind;
use http::{Request, Response};
use opentelemetry_http::{HeaderExtractor, HeaderInjector};
use opentelemetry_semantic_conventions::attribute::{
    EXCEPTION_MESSAGE, OTEL_STATUS_CODE, RPC_GRPC_STATUS_CODE,
};
use pin_project::pin_project;
use std::{
    fmt::Display,
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tower::{Layer, Service};
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// [`Layer`] that adds tracing to a [`Service`] that handles gRPC requests.
#[derive(Clone, Debug)]
pub struct GrpcLayer {
    level: Level,
    kind: SpanKind,
}

impl GrpcLayer {
    /// [`Span`]s are constructed at the given level from server side.
    pub fn server(level: Level) -> Self {
        Self {
            level,
            kind: SpanKind::Server,
        }
    }

    /// [`Span`]s are constructed at the given level from client side.
    pub fn client(level: Level) -> Self {
        Self {
            level,
            kind: SpanKind::Client,
        }
    }
}

impl<S> Layer<S> for GrpcLayer {
    type Service = GrpcService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GrpcService {
            inner,
            level: self.level,
            kind: self.kind,
        }
    }
}

/// Middleware that adds tracing to a [`Service`] that handles gRPC requests.
#[derive(Clone, Debug)]
pub struct GrpcService<S> {
    inner: S,
    level: Level,
    kind: SpanKind,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for GrpcService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Error: Display,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let span = self.make_request_span(&mut req);
        let inner = {
            let _enter = span.enter();
            self.inner.call(req)
        };

        ResponseFuture {
            inner,
            span,
            kind: self.kind,
        }
    }
}

impl<S> GrpcService<S> {
    /// Creates a new [`Span`] for the given request.
    fn make_request_span<B>(&self, request: &mut Request<B>) -> Span {
        let Self { level, kind, .. } = self;

        macro_rules! make_span {
            ($level:expr) => {{
                use tracing::field::Empty;

                tracing::span!(
                    $level,
                    "GRPC",
                    "exception.message" = Empty,
                    "otel.kind" = tracing::field::debug(kind),
                    "otel.name" = Empty,
                    "otel.status_code" = Empty,
                    "rpc.grpc.status_code" = Empty,
                    "rpc.method" = Empty,
                    "rpc.service" = Empty,
                    "rpc.system" = "grpc",
                )
            }};
        }

        let span = match *level {
            Level::ERROR => make_span!(Level::ERROR),
            Level::WARN => make_span!(Level::WARN),
            Level::INFO => make_span!(Level::INFO),
            Level::DEBUG => make_span!(Level::DEBUG),
            Level::TRACE => make_span!(Level::TRACE),
        };

        let path = request.uri().path();
        let name = path.trim_start_matches('/');
        span.record("otel.name", name);
        if let Some((service, method)) = name.split_once('/') {
            span.record("rpc.service", service);
            span.record("rpc.method", method);
        }

        let capture_request_headers = kind.capture_request_headers();

        for (header_name, header_value) in request.headers().iter() {
            let header_name = header_name.as_str().to_lowercase();
            if capture_request_headers.contains(&header_name) {
                if let Ok(attribute_value) = header_value.to_str() {
                    // attribute::RPC_GRPC_REQUEST_METADATA
                    let attribute_name = format!("rpc.grpc.request.metadata.{header_name}");
                    span.set_attribute(attribute_name, attribute_value.to_owned());
                }
            }
        }

        match kind {
            SpanKind::Client => {
                let context = span.context();
                opentelemetry::global::get_text_map_propagator(|injector| {
                    injector.inject_context(&context, &mut HeaderInjector(request.headers_mut()));
                });
            }
            SpanKind::Server => {
                let context = opentelemetry::global::get_text_map_propagator(|extractor| {
                    extractor.extract(&HeaderExtractor(request.headers()))
                });
                let _ = span.set_parent(context);
            }
        }

        span
    }
}

/// Response future for [`GrpcService`].
#[pin_project]
pub struct ResponseFuture<F> {
    #[pin]
    inner: F,
    span: Span,
    kind: SpanKind,
}

impl<F, ResBody, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    E: Display,
{
    type Output = Result<Response<ResBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _enter = this.span.enter();

        match ready!(this.inner.poll(cx)) {
            Ok(response) => {
                Self::record_response(this.span, *this.kind, &response);
                Poll::Ready(Ok(response))
            }
            Err(err) => {
                Self::record_error(this.span, &err);
                Poll::Ready(Err(err))
            }
        }
    }
}

impl<F, ResBody, E> ResponseFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    E: Display,
{
    /// Records fields associated to the response.
    fn record_response<B>(span: &Span, kind: SpanKind, response: &Response<B>) {
        let capture_response_headers = kind.capture_response_headers();

        for (header_name, header_value) in response.headers().iter() {
            let header_name = header_name.as_str().to_lowercase();
            if capture_response_headers.contains(&header_name) {
                if let Ok(attribute_value) = header_value.to_str() {
                    let attribute_name: String =
                        format!("rpc.grpc.response.metadata.{header_name}");
                    span.set_attribute(attribute_name, attribute_value.to_owned());
                }
            }
        }

        if let Some(header_value) = response.headers().get("grpc-status") {
            if let Ok(header_value) = header_value.to_str() {
                if let Ok(status_code) = header_value.parse::<i32>() {
                    span.record(RPC_GRPC_STATUS_CODE, status_code);
                }
            }
        } else {
            span.record(RPC_GRPC_STATUS_CODE, 0);
        }
    }

    /// Records the error message.
    fn record_error(span: &Span, err: &E) {
        span.record(OTEL_STATUS_CODE, "ERROR");
        span.record(EXCEPTION_MESSAGE, err.to_string());
    }
}
