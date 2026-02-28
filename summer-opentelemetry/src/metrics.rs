//! Middleware that adds metrics to a [`Service`] that handles HTTP requests.
//! refs: https://opentelemetry.io/docs/specs/semconv/http/http-metrics/

use crate::util::http as http_util;
use http::{Request, Response};
use http_body::Body;
use opentelemetry::{
    metrics::{Histogram, Meter, UpDownCounter},
    KeyValue,
};
use opentelemetry_semantic_conventions::{
    attribute::{HTTP_REQUEST_METHOD, SERVER_ADDRESS},
    metric::{
        HTTP_CLIENT_ACTIVE_REQUESTS, HTTP_CLIENT_REQUEST_BODY_SIZE, HTTP_CLIENT_REQUEST_DURATION,
        HTTP_CLIENT_RESPONSE_BODY_SIZE, HTTP_SERVER_ACTIVE_REQUESTS, HTTP_SERVER_REQUEST_BODY_SIZE,
        HTTP_SERVER_REQUEST_DURATION, HTTP_SERVER_RESPONSE_BODY_SIZE,
    },
    trace::{
        ERROR_TYPE, HTTP_RESPONSE_STATUS_CODE, HTTP_ROUTE, NETWORK_PROTOCOL_NAME,
        NETWORK_PROTOCOL_VERSION, SERVER_PORT,
    },
};
use pin_project::pin_project;
use std::{
    fmt::Display,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
    time::Instant,
};
use tower_layer::Layer;
use tower_service::Service;

#[derive(Debug)]
struct MetricsRecord {
    request_duration: Histogram<f64>,
    active_requests: UpDownCounter<i64>,
    request_body_size: Histogram<u64>,
    response_body_size: Histogram<u64>,
}

impl MetricsRecord {
    fn server(meter: &Meter) -> Self {
        Self {
            request_duration: meter
                .f64_histogram(HTTP_SERVER_REQUEST_DURATION)
                .with_description("Duration of HTTP server requests")
                .with_unit("s")
                .with_boundaries(vec![
                    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
                ])
                .build(),
            active_requests: meter
                .i64_up_down_counter(HTTP_SERVER_ACTIVE_REQUESTS)
                .with_description("Number of active HTTP server requests")
                .with_unit("{request}")
                .build(),
            request_body_size: meter
                .u64_histogram(HTTP_SERVER_REQUEST_BODY_SIZE)
                .with_description("Size of HTTP server request body")
                .with_unit("By")
                .build(),
            response_body_size: meter
                .u64_histogram(HTTP_SERVER_RESPONSE_BODY_SIZE)
                .with_description("Size of HTTP server response body")
                .with_unit("By")
                .build(),
        }
    }

    fn client(meter: &Meter) -> Self {
        Self {
            request_duration: meter
                .f64_histogram(HTTP_CLIENT_REQUEST_DURATION)
                .with_description("Duration of HTTP client requests")
                .with_unit("s")
                .with_boundaries(vec![
                    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
                ])
                .build(),
            request_body_size: meter
                .u64_histogram(HTTP_CLIENT_REQUEST_BODY_SIZE)
                .with_description("Size of HTTP client request body")
                .with_unit("By")
                .build(),
            response_body_size: meter
                .u64_histogram(HTTP_CLIENT_RESPONSE_BODY_SIZE)
                .with_description("Size of HTTP client response body")
                .with_unit("By")
                .build(),
            active_requests: meter
                .i64_up_down_counter(HTTP_CLIENT_ACTIVE_REQUESTS)
                .with_description("Number of active HTTP client requests")
                .with_unit("{request}")
                .build(),
        }
    }
}

/// [`Layer`] that adds tracing to a [`Service`] that handles HTTP requests.
#[derive(Clone, Debug)]
pub struct HttpLayer {
    record: Arc<MetricsRecord>,
}

impl HttpLayer {
    /// Metrics are recorded from server side.
    pub fn server(meter: &Meter) -> Self {
        let record = MetricsRecord::server(meter);
        Self {
            record: Arc::new(record),
        }
    }

    /// Metrics are recorded from client side.
    pub fn client(meter: &Meter) -> Self {
        let record = MetricsRecord::client(meter);
        Self {
            record: Arc::new(record),
        }
    }
}

impl<S> Layer<S> for HttpLayer {
    type Service = Http<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Http {
            inner,
            record: Arc::clone(&self.record),
        }
    }
}

/// Middleware that adds tracing to a [`Service`] that handles HTTP requests.
#[derive(Clone, Debug)]
pub struct Http<S> {
    inner: S,
    record: Arc<MetricsRecord>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for Http<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Error: Display,
    ReqBody: Body,
    ResBody: Body,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let state = ResponseMetricState::new(&req);
        let record = Arc::clone(&self.record);
        let inner = self.inner.call(req);

        record
            .active_requests
            .add(1, state.active_requests_attributes());

        ResponseFuture {
            inner,
            record,
            state,
        }
    }
}

/// Response future for [`Http`].
#[pin_project]
pub struct ResponseFuture<F> {
    #[pin]
    inner: F,
    record: Arc<MetricsRecord>,
    state: ResponseMetricState,
}

impl<F, ResBody, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    ResBody: Body,
    E: Display,
{
    type Output = Result<Response<ResBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let inner_response = ready!(this.inner.poll(cx));
        let duration = this.state.elapsed_seconds();

        this.state.push_response_attributes(&inner_response);

        this.record
            .request_duration
            .record(duration, this.state.attributes());

        this.record
            .active_requests
            .add(-1, this.state.active_requests_attributes());

        if let Some(request_body_size) = this.state.request_body_size {
            this.record
                .request_body_size
                .record(request_body_size, this.state.attributes());
        }

        if let Ok(response) = inner_response.as_ref() {
            if let Some(response_size) = http_util::http_response_size(response) {
                this.record
                    .response_body_size
                    .record(response_size, this.state.attributes());
            }
        }

        Poll::Ready(inner_response)
    }
}

struct ResponseMetricState {
    start: Instant,
    /// The size of the request body.
    request_body_size: Option<u64>,
    /// Attributes to add to the metrics.
    attributes: Vec<KeyValue>,
    /// The number of attributes that are used for only for active requests counter.
    active_requests_attributes: usize,
}

impl ResponseMetricState {
    fn new<B: Body>(req: &Request<B>) -> Self {
        let start = Instant::now();

        let request_body_size = http_util::http_request_size(req);

        let active_requests_attributes;
        let attributes = {
            let mut attributes = vec![];

            let http_method = http_util::http_method(req.method());
            attributes.push(KeyValue::new(HTTP_REQUEST_METHOD, http_method));

            if let Some(server_address) = req.uri().host() {
                attributes.push(KeyValue::new(SERVER_ADDRESS, server_address.to_string()));
            }

            if let Some(server_port) = req.uri().port_u16() {
                attributes.push(KeyValue::new(SERVER_PORT, server_port as i64));
            }

            active_requests_attributes = attributes.len();

            attributes.push(KeyValue::new(NETWORK_PROTOCOL_NAME, "http"));

            if let Some(http_version) = http_util::http_version(req.version()) {
                attributes.push(KeyValue::new(NETWORK_PROTOCOL_VERSION, http_version));
            }

            if let Some(http_route) = http_util::http_route(req) {
                attributes.push(KeyValue::new(HTTP_ROUTE, http_route.to_string()));
            }

            attributes
        };

        Self {
            start,
            request_body_size,
            attributes,
            active_requests_attributes,
        }
    }

    fn push_response_attributes<B, E>(&mut self, res: &Result<Response<B>, E>)
    where
        E: Display,
    {
        match res {
            Ok(response) => {
                self.attributes.push(KeyValue::new(
                    HTTP_RESPONSE_STATUS_CODE,
                    response.status().as_u16() as i64,
                ));
            }
            Err(err) => {
                self.attributes
                    .push(KeyValue::new(ERROR_TYPE, err.to_string()));
            }
        }
    }

    /// Returns the elapsed time since the request was created in seconds.
    fn elapsed_seconds(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Return the attributes for each metric.
    fn attributes(&self) -> &[KeyValue] {
        &self.attributes[..]
    }

    /// Returns the attributes used for active requests counter.
    fn active_requests_attributes(&self) -> &[KeyValue] {
        &self.attributes[..self.active_requests_attributes]
    }
}
