mod grpc;
mod http;

pub use crate::trace::grpc::GrpcLayer;
pub use crate::trace::http::HttpLayer;

use std::env::VarError;

/// Describes the relationship between the [`Span`] and the service producing the span.
#[derive(Clone, Copy, Debug)]
enum SpanKind {
    /// The span describes a request sent to some remote service.
    Client,
    /// The span describes the server-side handling of a request.
    Server,
}

impl SpanKind {
    /// refs: https://opentelemetry.io/docs/zero-code/java/agent/instrumentation/http/
    fn capture_request_headers(&self) -> Vec<String> {
        let var = match self {
            Self::Client => "OTEL_INSTRUMENTATION_HTTP_CLIENT_CAPTURE_REQUEST_HEADERS",
            Self::Server => "OTEL_INSTRUMENTATION_HTTP_SERVER_CAPTURE_REQUEST_HEADERS",
        };
        Self::split_env_headers_value(var)
    }

    /// refs: https://opentelemetry.io/docs/zero-code/java/agent/instrumentation/http/
    fn capture_response_headers(&self) -> Vec<String> {
        let var = match self {
            Self::Client => "OTEL_INSTRUMENTATION_HTTP_CLIENT_CAPTURE_RESPONSE_HEADERS",
            Self::Server => "OTEL_INSTRUMENTATION_HTTP_SERVER_CAPTURE_RESPONSE_HEADERS",
        };
        Self::split_env_headers_value(var)
    }

    #[inline]
    fn split_env_headers_value(var: &str) -> Vec<String> {
        match std::env::var(var) {
            Ok(headers) => headers
                .split(",")
                .map(|s| s.trim().to_lowercase())
                .collect(),
            Err(e) => {
                if let VarError::NotUnicode(value) = e {
                    tracing::warn!("{var} contains invalid unicode data:{value:?}")
                }
                vec![]
            }
        }
    }
}
