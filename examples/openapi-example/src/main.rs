mod garde_example;
mod validator_example;

use schemars::JsonSchema;
use serde::Serialize;
use summer::{auto_config, App};
use summer_web::axum::Json;
use summer_web::extractor::Path;
use summer_web::get_api;
use summer_web::problem_details::ProblemDetails;
use summer_web::ProblemDetails as ProblemDetailsMacro;
use summer_web::WebConfigurator;
use summer_web::WebPlugin;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new().add_plugin(WebPlugin).run().await;
}

// ProblemDetails error enum + extension fields example handlers

// The ProblemDetails derive macro auto-generates:
// 1. From<ApiErrors> for ProblemDetails — error conversion
// 2. IntoResponse — allows returning errors directly from Axum handlers
// 3. OpenAPI integration — for documentation generation
#[derive(Debug, thiserror::Error, ProblemDetailsMacro)]
pub enum ApiErrors {
    #[status_code(400)]
    #[error("Invalid input provided")]
    BadRequest,

    #[status_code(400)]
    #[problem_type("https://api.myapp.com/problems/email-validation")]
    #[title("Email Validation Failed")]
    #[detail("The provided email address is not valid")]
    #[error("Invalid email")]
    InvalidEmail,

    #[status_code(422)]
    #[problem_type("https://api.myapp.com/problems/validation-failed")]
    #[detail("The request data failed validation checks")]
    #[error("Validation Failed")]
    ValidationFailed,

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

    #[status_code(429)]
    #[problem_type("https://api.myapp.com/problems/rate-limit-exceeded")]
    #[detail("You have exceeded the maximum number of requests per minute")]
    #[error("Rate Limit Exceeded")]
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
        999 => Err(ApiErrors::NotFoundError),
        1000 => Err(ApiErrors::RateLimitExceeded),
        9999 => Err(ApiErrors::TeaPod(CustomErrorSchema {
            code: 418,
            message: "I'm a teapot".to_string(),
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
        return Err(ApiErrors::SqlxError(summer_sqlx::sqlx::Error::PoolTimedOut));
    }
    Ok(UserInfo::new(user_id, "Sample user info".to_string()))
}

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
            ProblemDetails::new(429)
            .with_type("https://api.example.com/problems/rate-limit-exceeded")
            .with_title("Rate Limit Exceeded")
            .with_detail("You have sent too many requests in a given amount of time")
            .with_extension("retryAfter", serde_json::json!(30))
            .with_extension("limit", serde_json::json!(100))
            .with_extension("remaining", serde_json::json!(0))
            .with_extension("resetAt", serde_json::json!("2026-03-22T12:00:00Z")),
        ),
        "payment" => Err(
            ProblemDetails::new(402)
            .with_type("https://api.example.com/problems/insufficient-balance")
            .with_title("Insufficient Balance")
            .with_detail("Your account balance is insufficient for this transaction")
            .with_extension("balance", serde_json::json!(50.00))
            .with_extension("required", serde_json::json!(99.99))
            .with_extension("currency", serde_json::json!("USD"))
            .with_extension("accountId", serde_json::json!("acc_12345")),
        ),
        "trace" => Err(
            ProblemDetails::new(500)
            .with_type("https://api.example.com/problems/internal-error")
            .with_title("Internal Server Error")
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
