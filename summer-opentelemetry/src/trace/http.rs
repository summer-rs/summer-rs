//! Middleware that adds tracing to a [`Service`] that handles HTTP requests.
//! https://opentelemetry.io/docs/specs/semconv/http/http-spans/

use super::SpanKind;
use crate::util::http as http_util;
use http::{HeaderName, HeaderValue, Request, Response};
use opentelemetry::trace::TraceContextExt;
use opentelemetry_http::{HeaderExtractor, HeaderInjector};
use opentelemetry_semantic_conventions::{
    attribute::{EXCEPTION_MESSAGE, HTTP_RESPONSE_STATUS_CODE, OTEL_STATUS_CODE},
    trace::HTTP_ROUTE,
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

/// [`Layer`] that adds tracing to a [`Service`] that handles HTTP requests.
#[derive(Clone, Debug)]
pub struct HttpLayer {
    level: Level,
    kind: SpanKind,
    export_trace_id: bool,
}

impl HttpLayer {
    /// [`Span`]s are constructed at the given level from server side.
    pub fn server(level: Level) -> Self {
        Self {
            level,
            kind: SpanKind::Server,
            export_trace_id: false,
        }
    }

    /// [`Span`]s are constructed at the given level from client side.
    pub fn client(level: Level) -> Self {
        Self {
            level,
            kind: SpanKind::Client,
            export_trace_id: false,
        }
    }

    pub fn export_trace_id(mut self, export_trace_id: bool) -> Self {
        self.export_trace_id = export_trace_id;
        self
    }
}

impl<S> Layer<S> for HttpLayer {
    type Service = HttpService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpService {
            inner,
            level: self.level,
            kind: self.kind,
            export_trace_id: self.export_trace_id,
        }
    }
}

/// Middleware that adds tracing to a [`Service`] that handles HTTP requests.
#[derive(Clone, Debug)]
pub struct HttpService<S> {
    inner: S,
    level: Level,
    kind: SpanKind,
    export_trace_id: bool,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for HttpService<S>
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
            export_trace_id: self.export_trace_id,
        }
    }
}

impl<S> HttpService<S> {
    /// Creates a new [`Span`] for the given request.
    fn make_request_span<B>(&self, request: &mut Request<B>) -> Span {
        let Self { level, kind, .. } = self;

        // attribute::HTTP_REQUEST_METHOD
        macro_rules! make_span {
            ($level:expr) => {{
                use tracing::field::Empty;

                tracing::span!(
                    $level,
                    "HTTP",
                    "exception.message" = Empty,
                    "http.request.method" = tracing::field::display(request.method()),
                    "http.response.status_code" = Empty,
                    "network.protocol.name" = "http",
                    "network.protocol.version" = tracing::field::debug(request.version()),
                    "otel.kind" = tracing::field::debug(kind),
                    "otel.status_code" = Empty,
                    "url.full" = tracing::field::display(request.uri()),
                    "url.path" = request.uri().path(),
                    "url.query" = request.uri().query(),
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

        let capture_request_headers = kind.capture_request_headers();

        for (header_name, header_value) in request.headers().iter() {
            let header_name = header_name.as_str().to_lowercase();
            if capture_request_headers.contains(&header_name) {
                if let Ok(attribute_value) = header_value.to_str() {
                    // attribute::HTTP_REQUEST_HEADER
                    let attribute_name = format!("http.request.header.{header_name}");
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
                if let Some(http_route) = http_util::http_route(request) {
                    span.record(HTTP_ROUTE, http_route);
                }
                let context = opentelemetry::global::get_text_map_propagator(|extractor| {
                    extractor.extract(&HeaderExtractor(request.headers()))
                });
                let _ = span.set_parent(context);
            }
        }

        span
    }
}

/// Response future for [`Http`].
#[pin_project]
pub struct ResponseFuture<F> {
    #[pin]
    inner: F,
    span: Span,
    kind: SpanKind,
    export_trace_id: bool,
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
            Ok(mut response) => {
                Self::record_response(this.span, *this.kind, &response);
                if *this.export_trace_id {
                    let trace_id = this
                        .span
                        .context()
                        .span()
                        .span_context()
                        .trace_id()
                        .to_string();
                    if let Ok(value) = HeaderValue::from_str(&trace_id) {
                        response
                            .headers_mut()
                            .insert(HeaderName::from_static("x-trace-id"), value);
                    }
                }
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
        span.record(HTTP_RESPONSE_STATUS_CODE, response.status().as_u16() as i64);

        let capture_response_headers = kind.capture_response_headers();

        for (header_name, header_value) in response.headers().iter() {
            let header_name = header_name.as_str().to_lowercase();
            if capture_response_headers.contains(&header_name) {
                if let Ok(attribute_value) = header_value.to_str() {
                    let attribute_name: String = format!("http.response.header.{header_name}");
                    span.set_attribute(attribute_name, attribute_value.to_owned());
                }
            }
        }

        if let SpanKind::Client = kind {
            if response.status().is_client_error() {
                span.record(OTEL_STATUS_CODE, "ERROR");
            }
        }
        if response.status().is_server_error() {
            span.record(OTEL_STATUS_CODE, "ERROR");
        }
    }

    /// Records the error message.
    fn record_error(span: &Span, err: &E) {
        span.record(OTEL_STATUS_CODE, "ERROR");
        span.record(EXCEPTION_MESSAGE, err.to_string());
    }
}
