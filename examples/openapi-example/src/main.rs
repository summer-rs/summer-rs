use schemars::JsonSchema;
use serde::Serialize;
use summer::{auto_config, App};
use summer_sqlx::SqlxPlugin;
use summer_web::axum::Json;
use summer_web::extractor::Path;
use summer_web::get_api;
use summer_web::WebPlugin;
use summer_web::WebConfigurator;
use summer_web::ProblemDetails as ProblemDetailsMacro;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SqlxPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await;
}

// ProblemDetails 宏会自动生成：
// 1. From<ApiErrors> for ProblemDetails - 用于错误转换
// 2. IntoResponse - 用于在 Axum 处理器中直接返回错误
// 3. OpenAPI 集成 - 用于文档生成
// 因此不需要手动实现这些 trait！
#[derive(Debug, thiserror::Error, ProblemDetailsMacro)]
pub enum ApiErrors {
    // 基本用法：使用 about:blank 作为默认 problem_type
    #[status_code(400)]
    #[error("Invalid input provided")]
    BadRequest,

    // 部分自定义：自定义 title 和 detail (显式使用 title 属性)
    #[status_code(400)]
    #[problem_type("https://api.myapp.com/problems/email-validation")]
    #[title("Email Validation Failed")]
    #[detail("The provided email address is not valid")]
    #[error("Invalid email")]
    InvalidEmail,

    // 使用 error 属性作为 title (兼容性功能)
    #[status_code(422)]
    #[problem_type("https://api.myapp.com/problems/validation-failed")]
    #[detail("The request data failed validation checks")]
    #[error("Validation Failed")]  // 这个会自动用作 title
    ValidationFailed,

    // 完全自定义：所有字段都自定义
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

    // 自定义状态码：429 Too Many Requests (使用 error 属性作为 title)
    #[status_code(429)]
    #[problem_type("https://api.myapp.com/problems/rate-limit-exceeded")]
    #[detail("You have exceeded the maximum number of requests per minute")]
    #[error("Rate Limit Exceeded")]  // 这个会自动用作 title
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
