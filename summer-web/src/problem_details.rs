//! RFC 9457 Problem Details for HTTP APIs
//!
//! This module implements the [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) standard
//! (which obsoletes RFC 7807) for structured HTTP error responses.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use summer_web::problem_details::ProblemDetails;
//!
//! // Simple error — title auto-derived from HTTP status phrase
//! let problem = ProblemDetails::new(404).with_detail("User 42 was not found");
//!
//! // Validation error with field-level violations
//! use summer_web::problem_details::{Violation, ViolationLocation};
//! let problem = ProblemDetails::validation_error(vec![
//!     Violation::body("email", "must be a valid email address"),
//!     Violation::query("page", "must be at least 1"),
//! ]);
//!
//! // Custom error with extensions (RFC 9457 §5)
//! let problem = ProblemDetails::new(429)
//!     .with_type("https://api.example.com/problems/rate-limit")
//!     .with_title("Rate Limit Exceeded")
//!     .with_detail("Too many requests")
//!     .with_extension("retryAfter", serde_json::json!(30));
//! ```

use axum::response::IntoResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "openapi")]
use aide::openapi::{MediaType, Operation, ReferenceOr, Response, SchemaObject, StatusCode};

/// Trait for providing variant information for Problem Details OpenAPI documentation
#[cfg(feature = "openapi")]
pub trait ProblemDetailsVariantInfo {
    fn get_variant_info(variant_name: &str) -> Option<(u16, String, Option<schemars::Schema>)>;
}

/// Generate Problem Details schema for OpenAPI documentation
#[cfg(feature = "openapi")]
pub fn problem_details_schema() -> schemars::Schema {
    ProblemDetails::json_schema(&mut schemars::SchemaGenerator::default())
}

/// Register error response by variant for OpenAPI documentation
#[cfg(feature = "openapi")]
pub fn register_error_response_by_variant<T>(
    _ctx: &mut aide::generate::GenContext,
    operation: &mut Operation,
    variant_path: &str,
) where
    T: ProblemDetailsVariantInfo,
{
    let variant_name = variant_path.split("::").last().unwrap_or(variant_path);

    let Some((status_code, description, _schema_opt)) = T::get_variant_info(variant_name) else {
        tracing::warn!(
            "Variant '{}' not found in error type '{}' when registering OpenAPI responses",
            variant_name,
            std::any::type_name::<T>()
        );
        return;
    };

    let problem_type = format!(
        "about:blank/{}",
        variant_name.to_lowercase().replace("::", "-")
    );
    let example = serde_json::json!({
        "type": problem_type,
        "title": format!("{} Error", variant_name),
        "status": status_code,
        "detail": format!("{} occurred", variant_name)
    });

    let response = Response {
        description,
        content: {
            let mut content = indexmap::IndexMap::new();
            let media_type = MediaType {
                schema: Some(SchemaObject {
                    json_schema: problem_details_schema(),
                    example: Some(example),
                    external_docs: None,
                }),
                ..Default::default()
            };
            content.insert("application/problem+json".to_string(), media_type.clone());
            content.insert("application/json".to_string(), media_type);
            content
        },
        ..Default::default()
    };

    if operation.responses.is_none() {
        operation.responses = Some(Default::default());
    }

    let responses = operation.responses.as_mut().unwrap();
    let status_code_key = StatusCode::Code(status_code);

    if let Some(existing) = responses.responses.get_mut(&status_code_key) {
        if let ReferenceOr::Item(existing_response) = existing {
            if existing_response.description != response.description {
                existing_response.description = format!(
                    "{}\n- {}",
                    existing_response.description, response.description
                );
            }
        }
    } else {
        responses
            .responses
            .insert(status_code_key, ReferenceOr::Item(response));
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Violation types
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The location in the HTTP request where a validation violation occurred.
///
/// Covers all standard HTTP request parts where user input can appear.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationLocation {
    Body,
    Query,
    Path,
    Header,
    Form,
}

/// A single validation violation describing which field failed and why.
///
/// Follows the Quarkus-style violation model as proposed in
/// [summer-rs#224](https://github.com/summer-rs/summer-rs/issues/224).
///
/// ```json
/// { "field": "email", "in": "body", "message": "must be a valid email address" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Violation {
    /// The field that failed validation (e.g. "name", "items[0].email")
    pub field: String,
    /// Where in the request the field is located
    #[serde(rename = "in")]
    pub location: ViolationLocation,
    /// A human-readable description of the violation
    pub message: String,
}

impl Violation {
    pub fn new(
        field: impl Into<String>,
        location: ViolationLocation,
        message: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            location,
            message: message.into(),
        }
    }

    /// Create a violation for a body field
    pub fn body(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, ViolationLocation::Body, message)
    }

    /// Create a violation for a query parameter
    pub fn query(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, ViolationLocation::Query, message)
    }

    /// Create a violation for a path parameter
    pub fn path(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, ViolationLocation::Path, message)
    }

    /// Create a violation for a header
    pub fn header(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, ViolationLocation::Header, message)
    }

    /// Create a violation for a form field
    pub fn form(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(field, ViolationLocation::Form, message)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ProblemDetails
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The well-known problem type URI for validation errors.
///
/// Used instead of `about:blank` so that a custom title ("Validation Error")
/// can be used without violating RFC 9457 §4.2.1, which requires `about:blank`
/// titles to match the HTTP status phrase exactly.
pub const VALIDATION_PROBLEM_TYPE: &str = "urn:problem-type:validation-error";

/// Problem Details for HTTP APIs (RFC 9457 / RFC 7807)
///
/// A standardized error response format as defined in
/// [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457).
///
/// # Construction patterns
///
/// ```rust,ignore
/// // From status code (title auto-derived from HTTP status phrase)
/// ProblemDetails::new(404).with_detail("User 42 was not found");
///
/// // Convenience constructors
/// ProblemDetails::bad_request("Missing required field: name");
/// ProblemDetails::not_found("user");
///
/// // Validation errors with field-level violations
/// ProblemDetails::validation_error(vec![
///     Violation::body("email", "must be a valid email address"),
/// ]);
///
/// // Custom problem type with extensions
/// ProblemDetails::new(429)
///     .with_type("https://api.example.com/problems/rate-limit")
///     .with_title("Rate Limit Exceeded")
///     .with_extension("retryAfter", serde_json::json!(30));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type.
    /// Defaults to `"about:blank"` per RFC 9457 §4.2.1.
    #[serde(rename = "type")]
    pub problem_type: String,

    /// A short, human-readable summary of the problem type.
    /// When `type` is `"about:blank"`, this MUST match the HTTP status phrase.
    pub title: String,

    /// The HTTP status code generated by the origin server.
    pub status: u16,

    /// A human-readable explanation specific to this occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    /// A URI reference identifying the specific occurrence of the problem.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Field-level validation violations (extension field).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub violations: Vec<Violation>,

    /// Additional problem-specific extension fields (RFC 9457 §5).
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ProblemDetails {
    // ── Core constructor ────────────────────────────────────────────

    /// Create a ProblemDetails from an HTTP status code.
    ///
    /// The `type` is set to `"about:blank"` and `title` is derived from the
    /// standard HTTP status phrase, conforming to RFC 9457 §4.2.1.
    pub fn new(status: u16) -> Self {
        let title = axum::http::StatusCode::from_u16(status)
            .map(|s| s.canonical_reason().unwrap_or("Unknown Error"))
            .unwrap_or("Unknown Error");
        Self {
            problem_type: "about:blank".to_string(),
            title: title.to_string(),
            status,
            detail: None,
            instance: None,
            violations: Vec::new(),
            extensions: HashMap::new(),
        }
    }

    // ── Builder methods ─────────────────────────────────────────────

    /// Override the problem type URI.
    pub fn with_type(mut self, problem_type: impl Into<String>) -> Self {
        self.problem_type = problem_type.into();
        self
    }

    /// Override the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the detail field.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the instance field.
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Add an extension field (RFC 9457 §5).
    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// Add a single validation violation.
    pub fn with_violation(
        mut self,
        field: impl Into<String>,
        location: ViolationLocation,
        message: impl Into<String>,
    ) -> Self {
        self.violations
            .push(Violation::new(field, location, message));
        self
    }

    /// Add multiple validation violations at once.
    pub fn with_violations(mut self, violations: Vec<Violation>) -> Self {
        self.violations.extend(violations);
        self
    }

    // ── Convenience constructors ────────────────────────────────────

    /// 400 Bad Request with a detail message.
    ///
    /// Uses `about:blank` with the standard "Bad Request" title per RFC 9457.
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(400).with_detail(detail)
    }

    /// 400 Validation Error with field-level violations.
    ///
    /// Uses a dedicated problem type URI so that the custom title
    /// "Validation Error" does not violate RFC 9457 §4.2.1.
    pub fn validation_error(violations: Vec<Violation>) -> Self {
        let count = violations.len();
        Self::new(400)
            .with_type(VALIDATION_PROBLEM_TYPE)
            .with_title("Validation Error")
            .with_detail(format!(
                "{count} validation error{} occurred",
                if count == 1 { "" } else { "s" }
            ))
            .with_violations(violations)
    }

    /// 400 Validation Error with a simple detail message (no violations).
    pub fn validation_error_simple(detail: impl Into<String>) -> Self {
        Self::new(400)
            .with_type(VALIDATION_PROBLEM_TYPE)
            .with_title("Validation Error")
            .with_detail(detail)
    }

    /// 401 Unauthorized.
    pub fn unauthorized() -> Self {
        Self::new(401)
            .with_detail("Authentication credentials are required to access this resource")
    }

    /// 403 Forbidden.
    pub fn forbidden() -> Self {
        Self::new(403).with_detail("You don't have permission to access this resource")
    }

    /// 404 Not Found for a named resource.
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::new(404).with_detail(format!("The requested {} was not found", resource.into()))
    }

    /// 500 Internal Server Error.
    pub fn internal_server_error() -> Self {
        Self::new(500)
            .with_detail("An unexpected error occurred while processing your request")
    }

    /// 503 Service Unavailable.
    pub fn service_unavailable() -> Self {
        Self::new(503).with_detail("The service is temporarily unavailable")
    }

    // ── Helpers ─────────────────────────────────────────────────────

    /// Get the HTTP status code.
    pub fn status_code(&self) -> axum::http::StatusCode {
        axum::http::StatusCode::from_u16(self.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// IntoResponse
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

// Task-local storage for current request URI
tokio::task_local! {
    static CURRENT_REQUEST_URI: String;
}

/// Get the current request URI from task-local storage
fn get_current_request_uri() -> Option<String> {
    CURRENT_REQUEST_URI.try_with(|uri| uri.clone()).ok()
}

/// Middleware to capture request URI for Problem Details.
///
/// When this middleware is active, any `ProblemDetails` response that lacks
/// an `instance` field will automatically include the request URI.
pub async fn capture_request_uri_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let uri = req.uri().to_string();
    CURRENT_REQUEST_URI
        .scope(uri, async move { next.run(req).await })
        .await
}

impl IntoResponse for ProblemDetails {
    fn into_response(mut self) -> axum::response::Response {
        let status = axum::http::StatusCode::from_u16(self.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        // Auto-fill instance from task-local URI if not set
        if self.instance.is_none() {
            if let Some(uri) = get_current_request_uri() {
                self.instance = Some(uri);
            }
        }

        (
            status,
            [("content-type", "application/problem+json")],
            axum::Json(self),
        )
            .into_response()
    }
}

#[cfg(feature = "openapi")]
impl aide::OperationOutput for ProblemDetails {
    type Inner = Self;

    fn operation_response(
        _ctx: &mut aide::generate::GenContext,
        _operation: &mut aide::openapi::Operation,
    ) -> Option<aide::openapi::Response> {
        None
    }

    fn inferred_responses(
        _ctx: &mut aide::generate::GenContext,
        _operation: &mut aide::openapi::Operation,
    ) -> Vec<(
        Option<aide::openapi::StatusCode>,
        aide::openapi::Response,
    )> {
        vec![]
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// validator integration (behind "validator" feature)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(feature = "validator")]
mod validator_support {
    use super::*;
    use ::validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

    /// Convert `validator::ValidationErrors` into a `ProblemDetails` with violations.
    impl ProblemDetails {
        pub fn from_validation_errors(
            errors: &ValidationErrors,
            location: ViolationLocation,
        ) -> Self {
            let mut violations = Vec::new();
            collect_violations(errors, &location, "", &mut violations);
            Self::validation_error(violations)
        }
    }

    /// Recursively collect violations from nested `ValidationErrors`.
    fn collect_violations(
        errors: &ValidationErrors,
        location: &ViolationLocation,
        prefix: &str,
        violations: &mut Vec<Violation>,
    ) {
        for (field, kind) in errors.errors() {
            let full_field = if prefix.is_empty() {
                field.to_string()
            } else {
                format!("{prefix}.{field}")
            };

            match kind {
                ValidationErrorsKind::Field(errs) => {
                    for err in errs {
                        let message = format_validation_message(err);
                        violations.push(Violation::new(
                            &full_field,
                            location.clone(),
                            message,
                        ));
                    }
                }
                ValidationErrorsKind::Struct(nested) => {
                    collect_violations(nested, location, &full_field, violations);
                }
                ValidationErrorsKind::List(map) => {
                    for (index, nested) in map {
                        let indexed = format!("{full_field}[{index}]");
                        collect_violations(nested, location, &indexed, violations);
                    }
                }
            }
        }
    }

    /// Turn a `validator::ValidationError` into a human-friendly message.
    fn format_validation_message(err: &ValidationError) -> String {
        // Prefer custom message if provided via #[validate(... message = "...")]
        if let Some(msg) = &err.message {
            return msg.to_string();
        }

        match err.code.as_ref() {
            "required" => "this field is required".to_string(),
            "email" => "must be a valid email address".to_string(),
            "url" => "must be a valid URL".to_string(),
            "phone" => "must be a valid phone number".to_string(),
            "credit_card" => "must be a valid credit card number".to_string(),
            "ip" => "must be a valid IP address".to_string(),
            "non_control_character" => "must not contain control characters".to_string(),
            "length" => {
                let min = err.params.get("min").and_then(|v| v.as_u64());
                let max = err.params.get("max").and_then(|v| v.as_u64());
                match (min, max) {
                    (Some(min), Some(max)) => format!("length must be between {min} and {max}"),
                    (Some(min), None) => format!("length must be at least {min}"),
                    (None, Some(max)) => format!("length must be at most {max}"),
                    _ => "invalid length".to_string(),
                }
            }
            "range" => {
                let min = err.params.get("min").and_then(|v| v.as_f64());
                let max = err.params.get("max").and_then(|v| v.as_f64());
                match (min, max) {
                    (Some(min), Some(max)) => format!("must be between {min} and {max}"),
                    (Some(min), None) => format!("must be at least {min}"),
                    (None, Some(max)) => format!("must be at most {max}"),
                    _ => "value out of range".to_string(),
                }
            }
            "regex" => "format is invalid".to_string(),
            "contains" => {
                if let Some(pattern) = err.params.get("pattern").and_then(|v| v.as_str()) {
                    format!("must contain '{pattern}'")
                } else {
                    "missing required content".to_string()
                }
            }
            "must_match" => {
                if let Some(other) = err.params.get("other").and_then(|v| v.as_str()) {
                    format!("must match field '{other}'")
                } else {
                    "fields do not match".to_string()
                }
            }
            "custom" => "validation failed".to_string(),
            code => format!("validation failed: {code}"),
        }
    }
}

#[cfg(feature = "garde")]
impl ProblemDetails {
    /// Convert `garde::Report` into a `ProblemDetails` with violations.
    ///
    /// `garde::Report` is a flat collection of `(Path, Error)` pairs,
    /// where `Path` already contains the full dotted/indexed field path
    /// (e.g. `"items[0].name"`), and `Error` carries the message.
    pub fn from_garde_report(report: &garde::Report, location: ViolationLocation) -> Self {
        let violations: Vec<Violation> = report
            .iter()
            .map(|(path, error)| {
                let field = path.to_string();
                let field = if field.is_empty() {
                    "__all__".to_string()
                } else {
                    field
                };
                Violation::new(field, location.clone(), error.to_string())
            })
            .collect();
        Self::validation_error(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_derives_title_from_status() {
        let p = ProblemDetails::new(404);
        assert_eq!(p.problem_type, "about:blank");
        assert_eq!(p.title, "Not Found");
        assert_eq!(p.status, 404);
    }

    #[test]
    fn test_new_400_title_is_bad_request() {
        let p = ProblemDetails::new(400);
        assert_eq!(p.title, "Bad Request");
    }

    #[test]
    fn test_bad_request_convenience() {
        let p = ProblemDetails::bad_request("Name is required");
        assert_eq!(p.status, 400);
        assert_eq!(p.problem_type, "about:blank");
        assert_eq!(p.title, "Bad Request");
        assert_eq!(p.detail, Some("Name is required".to_string()));
    }

    #[test]
    fn test_validation_error_uses_dedicated_type() {
        let p = ProblemDetails::validation_error(vec![
            Violation::body("name", "required"),
        ]);
        assert_eq!(p.status, 400);
        assert_eq!(p.problem_type, VALIDATION_PROBLEM_TYPE);
        assert_eq!(p.title, "Validation Error");
        assert_eq!(p.detail, Some("1 validation error occurred".to_string()));
        assert_eq!(p.violations.len(), 1);
    }

    #[test]
    fn test_validation_error_plural() {
        let p = ProblemDetails::validation_error(vec![
            Violation::body("a", "x"),
            Violation::query("b", "y"),
        ]);
        assert_eq!(p.detail, Some("2 validation errors occurred".to_string()));
    }

    #[test]
    fn test_builder_chain() {
        let p = ProblemDetails::new(429)
            .with_type("https://example.com/rate-limit")
            .with_title("Rate Limit Exceeded")
            .with_detail("Too many requests")
            .with_instance("/api/users")
            .with_extension("retryAfter", serde_json::json!(30));

        assert_eq!(p.problem_type, "https://example.com/rate-limit");
        assert_eq!(p.title, "Rate Limit Exceeded");
        assert_eq!(p.status, 429);
        assert_eq!(p.detail, Some("Too many requests".to_string()));
        assert_eq!(p.instance, Some("/api/users".to_string()));
        assert_eq!(
            p.extensions.get("retryAfter"),
            Some(&serde_json::json!(30))
        );
    }

    #[test]
    fn test_not_found() {
        let p = ProblemDetails::not_found("user");
        assert_eq!(p.status, 404);
        assert_eq!(p.title, "Not Found");
    }

    #[test]
    fn test_status_code_helper() {
        let p = ProblemDetails::new(418);
        assert_eq!(p.status_code(), axum::http::StatusCode::IM_A_TEAPOT);
    }

    #[test]
    fn test_into_response() {
        let p = ProblemDetails::not_found("user");
        let response = p.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_serialize_without_violations_omits_key() {
        let p = ProblemDetails::not_found("user");
        let json = serde_json::to_value(&p).unwrap();
        assert!(json.get("violations").is_none());
    }

    #[test]
    fn test_serialize_with_violations() {
        let p = ProblemDetails::validation_error(vec![
            Violation::body("email", "invalid"),
        ]);
        let json = serde_json::to_value(&p).unwrap();
        let arr = json["violations"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["field"], "email");
        assert_eq!(arr[0]["in"], "body");
        assert_eq!(arr[0]["message"], "invalid");
    }

    #[test]
    fn test_deserialize_without_violations() {
        let json = r#"{"type":"about:blank","title":"Not Found","status":404}"#;
        let p: ProblemDetails = serde_json::from_str(json).unwrap();
        assert!(p.violations.is_empty());
    }

    #[test]
    fn test_deserialize_with_violations() {
        let json = r#"{
            "type":"about:blank","title":"Bad Request","status":400,
            "violations":[
                {"field":"name","in":"body","message":"required"},
                {"field":"page","in":"query","message":"must be positive"}
            ]
        }"#;
        let p: ProblemDetails = serde_json::from_str(json).unwrap();
        assert_eq!(p.violations.len(), 2);
        assert_eq!(p.violations[0].location, ViolationLocation::Body);
        assert_eq!(p.violations[1].location, ViolationLocation::Query);
    }

    #[test]
    fn test_violation_shortcuts() {
        assert_eq!(Violation::body("a", "b").location, ViolationLocation::Body);
        assert_eq!(Violation::query("a", "b").location, ViolationLocation::Query);
        assert_eq!(Violation::path("a", "b").location, ViolationLocation::Path);
        assert_eq!(Violation::header("a", "b").location, ViolationLocation::Header);
        assert_eq!(Violation::form("a", "b").location, ViolationLocation::Form);
    }

    #[test]
    fn test_unauthorized() {
        let p = ProblemDetails::unauthorized();
        assert_eq!(p.status, 401);
    }

    #[test]
    fn test_forbidden() {
        let p = ProblemDetails::forbidden();
        assert_eq!(p.status, 403);
    }

    #[tokio::test]
    async fn test_automatic_uri_capture() {
        let test_uri = "/test/path".to_string();
        CURRENT_REQUEST_URI
            .scope(test_uri.clone(), async {
                let uri = get_current_request_uri();
                assert_eq!(uri, Some(test_uri));
            })
            .await;
    }
}
