use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use summer::{auto_config, App};
use summer_sqlx::SqlxPlugin;
use summer_web::axum::Json;
use summer_web::extractor::Path;
use summer_web::{get_api, post_api};
use summer_web::WebPlugin;
use summer_web::WebConfigurator;
use summer_web::ProblemDetails as ProblemDetailsMacro;
use summer_web::problem_details::ProblemDetails;
use validator::Validate;

mod validation;
use validation::{JsonWithProblem, ValidatedJson, ValidatedQuery, ValidatedPath};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        // .add_plugin(SqlxPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await;
}

// The ProblemDetails derive macro auto-generates:
// 1. From<ApiErrors> for ProblemDetails — error conversion
// 2. IntoResponse — allows returning errors directly from Axum handlers
// 3. OpenAPI integration — for documentation generation
// No need to implement these traits manually!
#[derive(Debug, thiserror::Error, ProblemDetailsMacro)]
pub enum ApiErrors {
    // Basic usage: uses about:blank as the default problem_type
    #[status_code(400)]
    #[error("Invalid input provided")]
    BadRequest,

    // Partial customization: custom title and detail (explicit title attribute)
    #[status_code(400)]
    #[problem_type("https://api.myapp.com/problems/email-validation")]
    #[title("Email Validation Failed")]
    #[detail("The provided email address is not valid")]
    #[error("Invalid email")]
    InvalidEmail,

    // Uses #[error] as title (compatibility feature)
    #[status_code(422)]
    #[problem_type("https://api.myapp.com/problems/validation-failed")]
    #[detail("The request data failed validation checks")]
    #[error("Validation Failed")]  // automatically used as title
    ValidationFailed,

    // Fully customized: all fields specified
    #[status_code(401)]
    #[problem_type("https://api.myapp.com/problems/authentication-required")]
    #[title("Authentication Required")]
    #[detail("You must be authenticated to access this resource")]
    #[instance("/auth/login")]
    #[error("Authentication required")]
    AuthenticationRequired,

    #[status_code(403)]
    #[problem_type("https://api.myapp.com/problems/access-denied")]
    #[error("Access denied")]
    AuthorizationError,

    #[status_code(404)]
    #[problem_type("https://api.myapp.com/problems/resource-not-found")]
    #[error("Resource not found")]
    NotFoundError,

    #[status_code(500)]
    #[problem_type("https://api.myapp.com/problems/database-error")]
    #[error(transparent)]
    SqlxError(#[from] summer_sqlx::sqlx::Error),

    #[status_code(418)]
    #[problem_type("https://api.myapp.com/problems/teapot-error")]
    #[error("TeaPod error occurred: {0:?}")]
    TeaPod(CustomErrorSchema),

    // Custom status code: 429 Too Many Requests (#[error] used as title)
    #[status_code(429)]
    #[problem_type("https://api.myapp.com/problems/rate-limit-exceeded")]
    #[detail("You have exceeded the maximum number of requests per minute")]
    #[error("Rate Limit Exceeded")]  // automatically used as title
    RateLimitExceeded,
}

#[derive(Debug, JsonSchema)]
pub struct CustomErrorSchema {
    pub code: u16,
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UserInfo {
    user_id: i64,
    user_info: String,
}

impl UserInfo {
    fn new(user_id: i64, user_info: String) -> Self {
        Self { user_id, user_info }
    }
}

/// Get user information (with automatic URI capture)
///
/// This endpoint demonstrates automatic URI capture for Problem Details instance field.
/// The middleware automatically captures the request URI and includes it in error responses.
///
/// @tag User
/// @status_codes ApiErrors::BadRequest, ApiErrors::ValidationFailed, ApiErrors::AuthenticationRequired, ApiErrors::NotFoundError, ApiErrors::RateLimitExceeded
#[get_api("/user-info/{id}")]
async fn user_info_api(Path(id): Path<u32>) -> Result<Json<UserInfo>, ApiErrors> {
    match id {
        0 => Err(ApiErrors::BadRequest),
        1 => Err(ApiErrors::InvalidEmail),
        2 => Err(ApiErrors::ValidationFailed),
        3 => Err(ApiErrors::AuthenticationRequired),
        4 => Err(ApiErrors::AuthorizationError),
        999 => Err(ApiErrors::NotFoundError), // Will automatically include "/user-info/999" as instance
        1000 => Err(ApiErrors::RateLimitExceeded),
        9999 => Err(ApiErrors::TeaPod(CustomErrorSchema {
            code: 418,
            message: "I'm a teapot".to_string()
        })),
        _ => {
            let user_info = fetch_user_info(id as i64).await;
            if let Ok(info) = user_info {
                Ok(Json(info))
            } else {
                Err(ApiErrors::NotFoundError)
            }
        }
    }
}

async fn fetch_user_info(user_id: i64) -> Result<UserInfo, ApiErrors> {
    let is_database_connected = true;
    if !is_database_connected {
        return Err(ApiErrors::SqlxError(
            summer_sqlx::sqlx::Error::PoolTimedOut,
        ));
    }

    // Simulate fetching user info from a database or external service
    Ok(UserInfo::new(user_id, "Sample user info".to_string()))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Validation example handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// These handlers use ValidatedJson / ValidatedQuery / ValidatedPath
// so that deserialization failures are returned as RFC 9457 ProblemDetails
// with field-level violations instead of axum's default plain-text errors.

#[derive(Debug, Deserialize, JsonSchema, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(range(min = 0, max = 150))]
    pub age: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema, Validate)]
pub struct ListUsersQuery {
    #[validate(range(min = 1))]
    pub page: Option<i32>,
    #[validate(range(min = 1, max = 100))]
    pub size: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema, Validate)]
pub struct UserIdPath {
    pub id: u32,
}

/// Request struct WITHOUT validator macros — only serde deserialization errors are caught.
/// Compare with `CreateUserRequest` which uses `#[validate(email)]`, `#[validate(length(...))]`, etc.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateOrderRequest {
    pub product_name: String,
    pub quantity: i32,
    pub email: String,
}

/// Create a new user (body validation via custom extractor)
///
/// If the JSON body is missing required fields (`name`, `email`) or
/// has wrong types, the `ValidatedJson` extractor automatically
/// returns a ProblemDetails response with body-level violations.
///
/// Try sending `{}` or `{"name": 123}` to see the violation output.
///
/// @tag Validation
#[post_api("/validation/users")]
async fn create_user(
    ValidatedJson(body): ValidatedJson<CreateUserRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "age": body.age,
    }))
}

/// List users (query parameter validation via custom extractor)
///
/// If a query parameter has the wrong type (e.g. `?page=abc`),
/// the `ValidatedQuery` extractor automatically returns a
/// ProblemDetails response with query-level violations.
///
/// @tag Validation
#[get_api("/validation/users")]
async fn list_users(
    ValidatedQuery(query): ValidatedQuery<ListUsersQuery>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "page": query.page.unwrap_or(1),
        "size": query.size.unwrap_or(20),
        "total": 0,
        "items": [],
    }))
}

/// Get user by ID (path parameter validation via custom extractor)
///
/// If the path parameter is not a valid u32 (e.g. `/users/abc`),
/// the `ValidatedPath` extractor automatically returns a
/// ProblemDetails response with path-level violations.
///
/// @tag Validation
#[get_api("/validation/users/{id}")]
async fn get_user_by_id(
    ValidatedPath(path): ValidatedPath<UserIdPath>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": path.id,
        "name": "Example User",
        "email": "user@example.com",
    }))
}

/// Create order — NO validator macros, only serde deserialization
///
/// This endpoint uses `JsonWithProblem<T>` instead of `ValidatedJson<T>`.
/// The struct `CreateOrderRequest` does NOT derive `Validate`,
/// so only Phase 1 (serde deserialization) errors are caught.
///
/// Compare with `POST /validation/users` which uses two-phase validation.
///
/// @tag Validation
#[post_api("/validation/orders-raw")]
async fn create_order_raw(
    JsonWithProblem(body): JsonWithProblem<CreateOrderRequest>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1001,
        "product_name": body.product_name,
        "quantity": body.quantity,
        "email": body.email,
    }))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Extension fields example
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// RFC 9457 allows adding custom extension fields beyond the standard
// fields (type, title, status, detail, instance). This is useful for
// providing machine-readable metadata like retry-after, account balance,
// rate-limit counters, trace IDs, etc.

/// Demonstrate ProblemDetails with custom extension fields (RFC 9457)
///
/// Use different `code` path values to trigger different error scenarios:
/// - `rate-limit` — 429 with retryAfter, limit, remaining extensions
/// - `payment`    — 402 with balance, required, currency extensions
/// - `trace`      — 500 with traceId, timestamp, node extensions
/// - anything else returns 200 OK
///
/// @tag Extensions
#[get_api("/extensions/example/{code}")]
async fn extension_example(
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, ProblemDetails> {
    match code.as_str() {
        "rate-limit" => Err(
            ProblemDetails::new(
                "https://api.example.com/problems/rate-limit-exceeded",
                "Rate Limit Exceeded",
                429,
            )
            .with_detail("You have sent too many requests in a given amount of time")
            .with_extension("retryAfter", serde_json::json!(30))
            .with_extension("limit", serde_json::json!(100))
            .with_extension("remaining", serde_json::json!(0))
            .with_extension("resetAt", serde_json::json!("2026-03-22T12:00:00Z")),
        ),
        "payment" => Err(
            ProblemDetails::new(
                "https://api.example.com/problems/insufficient-balance",
                "Insufficient Balance",
                402,
            )
            .with_detail("Your account balance is insufficient for this transaction")
            .with_extension("balance", serde_json::json!(50.00))
            .with_extension("required", serde_json::json!(99.99))
            .with_extension("currency", serde_json::json!("USD"))
            .with_extension("accountId", serde_json::json!("acc_12345")),
        ),
        "trace" => Err(
            ProblemDetails::new(
                "https://api.example.com/problems/internal-error",
                "Internal Server Error",
                500,
            )
            .with_detail("An unexpected error occurred while processing your request")
            .with_extension("traceId", serde_json::json!("abc-123-def-456"))
            .with_extension("timestamp", serde_json::json!("2026-03-22T10:30:00Z"))
            .with_extension("node", serde_json::json!("web-server-03")),
        ),
        _ => Ok(Json(serde_json::json!({
            "message": "success",
            "code": code,
        }))),
    }
}
