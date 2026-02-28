#![allow(missing_docs)]
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use summer::error::AppError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, WebError>;

/// <https://tools.ietf.org/html/rfc7231>
#[derive(Error, Debug)]
#[error("request error, status code is {status_code}: {msg}")]
pub struct KnownWebError {
    status_code: StatusCode,
    msg: String,
}

macro_rules! impl_known_status_error {
    (
        $(
            $(#[$docs:meta])*
            $lower_case:ident, $upper_case:ident,
        )+
    ) => {
        impl KnownWebError {
            pub fn new<S:Into<String>>(status_code: StatusCode, msg: S) -> Self {
                Self {
                    status_code,
                    msg: msg.into(),
                }
            }
        $(
            $(#[$docs])*
            pub fn $lower_case<S:Into<String>>(msg: S) -> Self {
                Self::new(StatusCode::$upper_case, msg)
            }
        )+
        }
    };
}

impl_known_status_error! (
    /// 200 OK
    /// [[RFC7231, Section 6.3.1](https://tools.ietf.org/html/rfc7231#section-6.3.1)]
    ok, OK,
    /// 201 Created
    /// [[RFC7231, Section 6.3.2](https://tools.ietf.org/html/rfc7231#section-6.3.2)]
    created, CREATED,
    /// 202 Accepted
    /// [[RFC7231, Section 6.3.3](https://tools.ietf.org/html/rfc7231#section-6.3.3)]
    accepted, ACCEPTED,
    /// 203 Non-Authoritative Information
    /// [[RFC7231, Section 6.3.4](https://tools.ietf.org/html/rfc7231#section-6.3.4)]
    non_authoritative_information, NON_AUTHORITATIVE_INFORMATION,
    /// 204 No Content
    /// [[RFC7231, Section 6.3.5](https://tools.ietf.org/html/rfc7231#section-6.3.5)]
    no_content, NO_CONTENT,
    /// 205 Reset Content
    /// [[RFC7231, Section 6.3.6](https://tools.ietf.org/html/rfc7231#section-6.3.6)]
    reset_content, RESET_CONTENT,
    /// 206 Partial Content
    /// [[RFC7233, Section 4.1](https://tools.ietf.org/html/rfc7233#section-4.1)]
    partial_content, PARTIAL_CONTENT,
    /// 207 Multi-Status
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    multi_status, MULTI_STATUS,
    /// 208 Already Reported
    /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
    already_reported, ALREADY_REPORTED,


    /// 226 IM Used
    /// [[RFC3229](https://tools.ietf.org/html/rfc3229)]
    im_used, IM_USED,


    /// 300 Multiple Choices
    /// [[RFC7231, Section 6.4.1](https://tools.ietf.org/html/rfc7231#section-6.4.1)]
    multiple_choices, MULTIPLE_CHOICES,
    /// 301 Moved Permanently
    /// [[RFC7231, Section 6.4.2](https://tools.ietf.org/html/rfc7231#section-6.4.2)]
    moved_permanently, MOVED_PERMANENTLY,
    /// 302 Found
    /// [[RFC7231, Section 6.4.3](https://tools.ietf.org/html/rfc7231#section-6.4.3)]
    found, FOUND,
    /// 303 See Other
    /// [[RFC7231, Section 6.4.4](https://tools.ietf.org/html/rfc7231#section-6.4.4)]
    see_other, SEE_OTHER,
    /// 304 Not Modified
    /// [[RFC7232, Section 4.1](https://tools.ietf.org/html/rfc7232#section-4.1)]
    not_modified, NOT_MODIFIED,
    /// 305 Use Proxy
    /// [[RFC7231, Section 6.4.5](https://tools.ietf.org/html/rfc7231#section-6.4.5)]
    use_proxy, USE_PROXY,
    /// 307 Temporary Redirect
    /// [[RFC7231, Section 6.4.7](https://tools.ietf.org/html/rfc7231#section-6.4.7)]
    temporary_redirect, TEMPORARY_REDIRECT,
    /// 308 Permanent Redirect
    /// [[RFC7238](https://tools.ietf.org/html/rfc7238)]
    permanent_redirect, PERMANENT_REDIRECT,


    /// 400 Bad Request
    /// [[RFC7231, Section 6.5.1](https://tools.ietf.org/html/rfc7231#section-6.5.1)]
    bad_request, BAD_REQUEST,
    /// 401 Unauthorized
    /// [[RFC7235, Section 3.1](https://tools.ietf.org/html/rfc7235#section-3.1)]
    unauthorized, UNAUTHORIZED,
    /// 402 Payment Required
    /// [[RFC7231, Section 6.5.2](https://tools.ietf.org/html/rfc7231#section-6.5.2)]
    payment_required, PAYMENT_REQUIRED,
    /// 403 Forbidden
    /// [[RFC7231, Section 6.5.3](https://tools.ietf.org/html/rfc7231#section-6.5.3)]
    forbidden, FORBIDDEN,
    /// 404 Not Found
    /// [[RFC7231, Section 6.5.4](https://tools.ietf.org/html/rfc7231#section-6.5.4)]
    not_found, NOT_FOUND,
    /// 405 Method Not Allowed
    /// [[RFC7231, Section 6.5.5](https://tools.ietf.org/html/rfc7231#section-6.5.5)]
    method_not_allowed, METHOD_NOT_ALLOWED,
    /// 406 Not Acceptable
    /// [[RFC7231, Section 6.5.6](https://tools.ietf.org/html/rfc7231#section-6.5.6)]
    not_acceptable, NOT_ACCEPTABLE,
    /// 407 Proxy Authentication Required
    /// [[RFC7235, Section 3.2](https://tools.ietf.org/html/rfc7235#section-3.2)]
    proxy_authentication_required, PROXY_AUTHENTICATION_REQUIRED,
    /// 408 Request Timeout
    /// [[RFC7231, Section 6.5.7](https://tools.ietf.org/html/rfc7231#section-6.5.7)]
    request_timeout, REQUEST_TIMEOUT,
    /// 409 Conflict
    /// [[RFC7231, Section 6.5.8](https://tools.ietf.org/html/rfc7231#section-6.5.8)]
    conflict, CONFLICT,
    /// 410 Gone
    /// [[RFC7231, Section 6.5.9](https://tools.ietf.org/html/rfc7231#section-6.5.9)]
    gone, GONE,
    /// 411 Length Required
    /// [[RFC7231, Section 6.5.10](https://tools.ietf.org/html/rfc7231#section-6.5.10)]
    length_required, LENGTH_REQUIRED,
    /// 412 Precondition Failed
    /// [[RFC7232, Section 4.2](https://tools.ietf.org/html/rfc7232#section-4.2)]
    precondition_failed, PRECONDITION_FAILED,
    /// 413 Payload Too Large
    /// [[RFC7231, Section 6.5.11](https://tools.ietf.org/html/rfc7231#section-6.5.11)]
    payload_too_large, PAYLOAD_TOO_LARGE,
    /// 414 URI Too Long
    /// [[RFC7231, Section 6.5.12](https://tools.ietf.org/html/rfc7231#section-6.5.12)]
    uri_too_long, URI_TOO_LONG,
    /// 415 Unsupported Media Type
    /// [[RFC7231, Section 6.5.13](https://tools.ietf.org/html/rfc7231#section-6.5.13)]
    unsupported_media_type, UNSUPPORTED_MEDIA_TYPE,
    /// 416 Range Not Satisfiable
    /// [[RFC7233, Section 4.4](https://tools.ietf.org/html/rfc7233#section-4.4)]
    range_not_satisfiable, RANGE_NOT_SATISFIABLE,
    /// 417 Expectation Failed
    /// [[RFC7231, Section 6.5.14](https://tools.ietf.org/html/rfc7231#section-6.5.14)]
    expectation_failed, EXPECTATION_FAILED,
    /// 418 I'm a teapot
    /// [curiously not registered by IANA but [RFC2324](https://tools.ietf.org/html/rfc2324)]
    im_a_teapot, IM_A_TEAPOT,


    /// 421 Misdirected Request
    /// [RFC7540, Section 9.1.2](https://tools.ietf.org/html/rfc7540#section-9.1.2)
    misdirected_request, MISDIRECTED_REQUEST,
    /// 422 Unprocessable Entity
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    unprocessable_entity, UNPROCESSABLE_ENTITY,
    /// 423 Locked
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    locked, LOCKED,
    /// 424 Failed Dependency
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    failed_dependency, FAILED_DEPENDENCY,


    /// 426 Upgrade Required
    /// [[RFC7231, Section 6.5.15](https://tools.ietf.org/html/rfc7231#section-6.5.15)]
    upgrade_required, UPGRADE_REQUIRED,


    /// 428 Precondition Required
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    precondition_required, PRECONDITION_REQUIRED,
    /// 429 Too Many Requests
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    too_many_requests, TOO_MANY_REQUESTS,


    /// 431 Request Header Fields Too Large
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    request_header_fields_too_large, REQUEST_HEADER_FIELDS_TOO_LARGE,


    /// 451 Unavailable For Legal Reasons
    /// [[RFC7725](https://tools.ietf.org/html/rfc7725)]
    unavailable_for_legal_reasons, UNAVAILABLE_FOR_LEGAL_REASONS,


    /// 500 Internal Server Error
    /// [[RFC7231, Section 6.6.1](https://tools.ietf.org/html/rfc7231#section-6.6.1)]
    internal_server_error, INTERNAL_SERVER_ERROR,
    /// 501 Not Implemented
    /// [[RFC7231, Section 6.6.2](https://tools.ietf.org/html/rfc7231#section-6.6.2)]
    not_implemented, NOT_IMPLEMENTED,
    /// 502 Bad Gateway
    /// [[RFC7231, Section 6.6.3](https://tools.ietf.org/html/rfc7231#section-6.6.3)]
    bad_gateway, BAD_GATEWAY,
    /// 503 Service Unavailable
    /// [[RFC7231, Section 6.6.4](https://tools.ietf.org/html/rfc7231#section-6.6.4)]
    service_unavailable, SERVICE_UNAVAILABLE,
    /// 504 Gateway Timeout
    /// [[RFC7231, Section 6.6.5](https://tools.ietf.org/html/rfc7231#section-6.6.5)]
    gateway_timeout, GATEWAY_TIMEOUT,
    /// 505 HTTP Version Not Supported
    /// [[RFC7231, Section 6.6.6](https://tools.ietf.org/html/rfc7231#section-6.6.6)]
    http_version_not_supported, HTTP_VERSION_NOT_SUPPORTED,
    /// 506 Variant Also Negotiates
    /// [[RFC2295](https://tools.ietf.org/html/rfc2295)]
    variant_also_negotiates, VARIANT_ALSO_NEGOTIATES,
    /// 507 Insufficient Storage
    /// [[RFC4918](https://tools.ietf.org/html/rfc4918)]
    insufficient_storage, INSUFFICIENT_STORAGE,
    /// 508 Loop Detected
    /// [[RFC5842](https://tools.ietf.org/html/rfc5842)]
    loop_detected, LOOP_DETECTED,


    /// 510 Not Extended
    /// [[RFC2774](https://tools.ietf.org/html/rfc2774)]
    not_extended, NOT_EXTENDED,
    /// 511 Network Authentication Required
    /// [[RFC6585](https://tools.ietf.org/html/rfc6585)]
    network_authentication_required, NETWORK_AUTHENTICATION_REQUIRED,

);

#[derive(Error, Debug)]
pub enum WebError {
    #[error(transparent)]
    ResponseStatusError(#[from] KnownWebError),

    #[error("get server config failed for typeof {0}, {1}")]
    ConfigDeserializeErr(&'static str, Box<AppError>),

    #[error(transparent)]
    ServerError(#[from] anyhow::Error),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        match self {
            Self::ResponseStatusError(e) => {
                tracing::warn!("handler error:{:?}", e);
                (e.status_code, e.msg)
            }
            _other => {
                tracing::error!("internal server error:{_other:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Something went wrong: {_other}"),
                )
            }
        }
        .into_response()
    }
}

#[cfg(feature = "openapi")]
impl aide::OperationOutput for WebError {
    type Inner = Self;

    fn operation_response(
        _ctx: &mut aide::generate::GenContext,
        _op: &mut aide::openapi::Operation,
    ) -> Option<aide::openapi::Response> {
        Some(aide::openapi::Response {
            description: "internal server error".to_string(),
            ..Default::default()
        })
    }

    fn inferred_responses(
        ctx: &mut aide::generate::GenContext,
        op: &mut aide::openapi::Operation,
    ) -> Vec<(Option<aide::openapi::StatusCode>, aide::openapi::Response)> {
        vec![(
            Some(aide::openapi::StatusCode::Code(500)),
            Self::operation_response(ctx, op).unwrap(),
        )]
    }
}
