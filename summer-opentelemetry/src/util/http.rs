use http::{Method, Request, Version};
use http_body::Body;

/// String representation of HTTP method
pub fn http_method(method: &Method) -> &'static str {
    match *method {
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::DELETE => "DELETE",
        Method::HEAD => "HEAD",
        Method::OPTIONS => "OPTIONS",
        Method::CONNECT => "CONNECT",
        Method::PATCH => "PATCH",
        Method::TRACE => "TRACE",
        _ => "_OTHER",
    }
}

/// String representation of network protocol version
pub fn http_version(version: Version) -> Option<&'static str> {
    match version {
        Version::HTTP_09 => Some("0.9"),
        Version::HTTP_10 => Some("1.0"),
        Version::HTTP_11 => Some("1.1"),
        Version::HTTP_2 => Some("2"),
        Version::HTTP_3 => Some("3"),
        _ => None,
    }
}

/// Get the size of the HTTP request body from the `Content-Length` header.
pub fn http_request_size<B: Body>(req: &Request<B>) -> Option<u64> {
    req.headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .or_else(|| req.body().size_hint().exact())
}

/// Get the size of the HTTP response body from the `Content-Length` header.
pub fn http_response_size<B: Body>(res: &http::Response<B>) -> Option<u64> {
    res.headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .or_else(|| res.body().size_hint().exact())
}

pub fn http_route<B>(req: &http::Request<B>) -> Option<&str> {
    use axum::extract::MatchedPath;
    req.extensions().get::<MatchedPath>().map(|matched_path| matched_path.as_str())
}