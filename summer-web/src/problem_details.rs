use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use axum::response::IntoResponse;

// OpenAPI related imports - only available when openapi feature is enabled
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
    use schemars::JsonSchema;
    crate::problem_details::ProblemDetails::json_schema(&mut schemars::SchemaGenerator::default())
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
        tracing::warn!("Variant '{}' not found in error type '{}' when registering OpenAPI responses", 
                      variant_name, std::any::type_name::<T>());
        return;
    };
    
    // Create Problem Details response
    let problem_type = format!("about:blank/{}", variant_name.to_lowercase().replace("::", "-"));
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
            content.insert("application/json".to_string(), media_type); // backward compatibility
            content
        },
        ..Default::default()
    };
    
    // Add response to operation
    if operation.responses.is_none() {
        operation.responses = Some(Default::default());
    }
    
    let responses = operation.responses.as_mut().unwrap();
    let status_code_key = StatusCode::Code(status_code);
    
    if let Some(existing) = responses.responses.get_mut(&status_code_key) {
        // Merge descriptions if response already exists
        if let ReferenceOr::Item(existing_response) = existing {
            if existing_response.description != response.description {
                existing_response.description = format!("{}\n- {}", existing_response.description, response.description);
            }
        }
    } else {
        responses.responses.insert(status_code_key, ReferenceOr::Item(response));
    }
}

/// The location in the HTTP request where a validation violation occurred.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ViolationLocation {
    Body,
    Query,
    Path,
}

/// A single validation violation describing which field failed and why.
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
    /// Create a new Violation
    pub fn new(field: impl Into<String>, location: ViolationLocation, message: impl Into<String>) -> Self {
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
}

/// Problem Details for HTTP APIs (RFC 9457 / RFC 7807)
///
/// This struct represents a standardized error response format as defined in
/// [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) (which obsoletes RFC 7807).
/// It provides a consistent way to communicate error information in HTTP APIs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type
    #[serde(rename = "type")]
    pub problem_type: String,
    
    /// A short, human-readable summary of the problem type
    pub title: String,
    
    /// The HTTP status code generated by the origin server
    pub status: u16,
    
    /// A human-readable explanation specific to this occurrence of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    
    /// A URI reference that identifies the specific occurrence of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,

    /// Field-level validation violations
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub violations: Vec<Violation>,

    /// Additional problem-specific extension fields
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ProblemDetails {
    /// Create a new ProblemDetails with required fields
    pub fn new(problem_type: impl Into<String>, title: impl Into<String>, status: u16) -> Self {
        Self {
            problem_type: problem_type.into(),
            title: title.into(),
            status,
            detail: None,
            instance: None,
            violations: Vec::new(),
            extensions: HashMap::new(),
        }
    }
    
    /// Set the detail field
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    
    /// Set the instance field
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }
    
    /// Add an extension field
    ///
    /// # Panics (debug only)
    /// Panics if `key` collides with a first-class field name.
    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let key = key.into();
        debug_assert!(
            !matches!(key.as_str(), "type" | "title" | "status" | "detail" | "instance" | "violations"),
            "extension key `{key}` conflicts with a first-class ProblemDetails field"
        );
        self.extensions.insert(key, value);
        self
    }

    /// Add a single validation violation
    pub fn with_violation(
        mut self,
        field: impl Into<String>,
        location: ViolationLocation,
        message: impl Into<String>,
    ) -> Self {
        self.violations.push(Violation::new(field, location, message));
        self
    }

    /// Add multiple validation violations at once
    pub fn with_violations(mut self, violations: Vec<Violation>) -> Self {
        self.violations.extend(violations);
        self
    }

    /// Create a validation error with violations
    ///
    /// Convenience constructor that builds a 400 response pre-populated with
    /// the given violations and an auto-generated detail message.
    ///
    /// Uses a dedicated problem type URI rather than `about:blank` so that the
    /// custom title "Validation Error" conforms to RFC 9457 §4.2.1 (which
    /// requires `about:blank` titles to match the HTTP status phrase).
    pub fn validation_error_with_violations(violations: Vec<Violation>) -> Self {
        let count = violations.len();
        Self::new("urn:problem-type:validation-error", "Validation Error", 400)
            .with_detail(format!("{count} validation error{} occurred", if count == 1 { "" } else { "s" }))
            .with_violations(violations)
    }
    
    /// Create a validation error problem
    pub fn validation_error(detail: impl Into<String>) -> Self {
        Self::new(
            "about:blank",
            "Validation Error",
            400,
        )
        .with_detail(detail)
    }
    
    /// Create an authentication error problem
    pub fn authentication_error() -> Self {
        Self::new(
            "about:blank",
            "Authentication Required",
            401,
        )
        .with_detail("Authentication credentials are required to access this resource")
    }
    
    /// Create an authorization error problem
    pub fn authorization_error() -> Self {
        Self::new(
            "about:blank",
            "Insufficient Permissions",
            403,
        )
        .with_detail("You don't have permission to access this resource")
    }
    
    /// Create a not found error problem
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::new(
            "about:blank",
            "Resource Not Found",
            404,
        )
        .with_detail(format!("The requested {} was not found", resource.into()))
    }
    
    /// Create an internal server error problem
    pub fn internal_server_error() -> Self {
        Self::new(
            "about:blank",
            "Internal Server Error",
            500,
        )
        .with_detail("An unexpected error occurred while processing your request")
    }
    
    /// Create a service unavailable error problem
    pub fn service_unavailable() -> Self {
        Self::new(
            "about:blank",
            "Service Unavailable",
            503,
        )
        .with_detail("The service is temporarily unavailable")
    }
    
    /// Create a custom problem with explicit URI
    pub fn custom_problem(problem_type: impl Into<String>, title: impl Into<String>, status: u16) -> Self {
        Self::new(
            problem_type,
            title,
            status,
        )
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(mut self) -> axum::response::Response {
        let status = axum::http::StatusCode::from_u16(self.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        // Try to get the current request URI from task-local storage
        if self.instance.is_none() {
            if let Some(uri) = get_current_request_uri() {
                self.instance = Some(uri);
            }
        }

        // Set the correct Content-Type for Problem Details
        (
            status,
            [("content-type", "application/problem+json")],
            axum::Json(self),
        ).into_response()
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
    ) -> Vec<(Option<aide::openapi::StatusCode>, aide::openapi::Response)> {
        vec![]
    }
}

// Task-local storage for current request URI
tokio::task_local! {
    static CURRENT_REQUEST_URI: String;
}

/// Get the current request URI from task-local storage
fn get_current_request_uri() -> Option<String> {
    CURRENT_REQUEST_URI.try_with(|uri| uri.clone()).ok()
}

/// Set the current request URI in task-local storage
pub fn set_current_request_uri(uri: String) {
    CURRENT_REQUEST_URI.scope(uri, async {
        // This will be available for the duration of the request
    });
}

/// Middleware to capture request URI for Problem Details
pub async fn capture_request_uri_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let uri = req.uri().to_string();
    
    // Run the rest of the request handling with the URI in task-local storage
    CURRENT_REQUEST_URI.scope(uri, async move {
        next.run(req).await
    }).await
}

/// Get the HTTP status code from ProblemDetails
impl ProblemDetails {
    pub fn status_code(&self) -> axum::http::StatusCode {
        axum::http::StatusCode::from_u16(self.status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_problem_details_creation() {
        let problem = ProblemDetails::new("https://example.com/problems/test", "Test Problem", 400)
            .with_detail("This is a test problem")
            .with_instance("/test/123")
            .with_extension("code", serde_json::Value::String("TEST_001".to_string()));
        
        assert_eq!(problem.problem_type, "https://example.com/problems/test");
        assert_eq!(problem.title, "Test Problem");
        assert_eq!(problem.status, 400);
        assert_eq!(problem.detail, Some("This is a test problem".to_string()));
        assert_eq!(problem.instance, Some("/test/123".to_string()));
        assert_eq!(problem.extensions.get("code"), Some(&serde_json::Value::String("TEST_001".to_string())));
    }
    
    #[test]
    fn test_validation_error() {
        // Test with default about:blank
        let problem = ProblemDetails::validation_error("Name is required");
        assert_eq!(problem.status, 400);
        assert_eq!(problem.title, "Validation Error");
        assert_eq!(problem.problem_type, "about:blank");
    }
    
    #[test]
    fn test_into_response() {
        let problem = ProblemDetails::not_found("user");
        let response = problem.into_response();
        
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }
    
    #[test]
    fn test_status_code() {
        let problem = ProblemDetails::validation_error("Test error");
        assert_eq!(problem.status_code(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_automatic_uri_capture() {
        // Test that URI is captured in task-local storage
        let test_uri = "/test/path".to_string();

        CURRENT_REQUEST_URI.scope(test_uri.clone(), async {
            let uri = get_current_request_uri();
            assert_eq!(uri, Some(test_uri));
        }).await;
    }

    #[test]
    fn test_violation_new() {
        let v = Violation::new("email", ViolationLocation::Body, "invalid format");
        assert_eq!(v.field, "email");
        assert_eq!(v.location, ViolationLocation::Body);
        assert_eq!(v.message, "invalid format");
    }

    #[test]
    fn test_violation_body_shortcut() {
        let v = Violation::body("name", "must not be null");
        assert_eq!(v.field, "name");
        assert_eq!(v.location, ViolationLocation::Body);
        assert_eq!(v.message, "must not be null");
    }

    #[test]
    fn test_violation_query_shortcut() {
        let v = Violation::query("page", "must be positive");
        assert_eq!(v.location, ViolationLocation::Query);
    }

    #[test]
    fn test_violation_path_shortcut() {
        let v = Violation::path("id", "must be a valid UUID");
        assert_eq!(v.location, ViolationLocation::Path);
    }

    #[test]
    fn test_problem_details_with_violations() {
        let problem = ProblemDetails::validation_error("validation failed")
            .with_violation("name", ViolationLocation::Body, "must not be null")
            .with_violation("age", ViolationLocation::Body, "must be positive");

        assert_eq!(problem.violations.len(), 2);
        assert_eq!(problem.violations[0].field, "name");
        assert_eq!(problem.violations[1].field, "age");
    }

    #[test]
    fn test_problem_details_with_violations_batch() {
        let violations = vec![
            Violation::body("a", "required"),
            Violation::query("b", "too long"),
        ];
        let problem = ProblemDetails::validation_error("err").with_violations(violations);
        assert_eq!(problem.violations.len(), 2);
    }

    #[test]
    fn test_serialize_without_violations_omits_key() {
        let problem = ProblemDetails::not_found("user");
        let json = serde_json::to_value(&problem).unwrap();
        assert!(json.get("violations").is_none(), "violations key should be absent when empty");
    }

    #[test]
    fn test_serialize_with_violations() {
        let problem = ProblemDetails::validation_error("bad input")
            .with_violation("email", ViolationLocation::Body, "invalid email format");

        let json = serde_json::to_value(&problem).unwrap();
        let violations = json.get("violations").expect("violations key should be present");
        let arr = violations.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["field"], "email");
        assert_eq!(arr[0]["in"], "body");
        assert_eq!(arr[0]["message"], "invalid email format");
    }

    #[test]
    fn test_deserialize_without_violations() {
        let json = r#"{"type":"about:blank","title":"Not Found","status":404}"#;
        let problem: ProblemDetails = serde_json::from_str(json).unwrap();
        assert!(problem.violations.is_empty());
    }

    #[test]
    fn test_deserialize_with_violations() {
        let json = r#"{
            "type":"about:blank",
            "title":"Validation Error",
            "status":400,
            "violations":[
                {"field":"name","in":"body","message":"must not be null"},
                {"field":"page","in":"query","message":"must be positive"}
            ]
        }"#;
        let problem: ProblemDetails = serde_json::from_str(json).unwrap();
        assert_eq!(problem.violations.len(), 2);
        assert_eq!(problem.violations[0].location, ViolationLocation::Body);
        assert_eq!(problem.violations[1].location, ViolationLocation::Query);
    }

    #[test]
    fn test_validation_error_with_violations() {
        let violations = vec![
            Violation::body("name", "must not be null"),
            Violation::body("items[0].email", "invalid email format"),
        ];
        let problem = ProblemDetails::validation_error_with_violations(violations);

        assert_eq!(problem.status, 400);
        assert_eq!(problem.problem_type, "urn:problem-type:validation-error");
        assert_eq!(problem.title, "Validation Error");
        assert_eq!(problem.detail, Some("2 validation errors occurred".to_string()));
        assert_eq!(problem.violations.len(), 2);
    }

    #[test]
    fn test_validation_error_with_violations_singular() {
        let violations = vec![Violation::body("name", "required")];
        let problem = ProblemDetails::validation_error_with_violations(violations);
        assert_eq!(problem.detail, Some("1 validation error occurred".to_string()));
    }
}