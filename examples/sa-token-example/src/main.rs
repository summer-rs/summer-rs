use anyhow::Context;
use summer::{auto_config, App};
use summer_sa_token::{
    sa_check_login, sa_check_permission, sa_check_permissions_and, sa_check_permissions_or,
    sa_check_role, sa_check_roles_and, sa_check_roles_or, sa_ignore, LoginIdExtractor,
    SaTokenAuthConfigurator, SaTokenPlugin, SaTokenState, StpUtil,
};
use summer_sea_orm::SeaOrmPlugin;
use summer_web::extractor::Component;
use summer_web::WebConfigurator;
use summer_web::{
    axum::response::IntoResponse, error::Result, extractor::Json, get, post, WebPlugin,
};

mod config;
mod models;
mod sea_orm_storage;

use models::*;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SaTokenPlugin)
        .add_plugin(WebPlugin)
        .add_plugin(SeaOrmPlugin)
        .sa_token_configure(config::SaTokenConfig)
        .run()
        .await
}


#[post("/login")]
async fn login(Json(req): Json<LoginRequest>) -> Result<impl IntoResponse> {
    if req.username.is_empty() || req.password.is_empty() {
        return Ok(Json(MessageResponse {
            message: "Username and password are required".to_string(),
        })
        .into_response());
    }

    let token = StpUtil::login(&req.username)
        .await
        .context("Login failed")?;

    let (roles, permissions) = get_user_roles_and_permissions(&req.username);

    StpUtil::set_roles(&req.username, roles)
        .await
        .context("Failed to set roles")?;

    StpUtil::set_permissions(&req.username, permissions)
        .await
        .context("Failed to set permissions")?;

    Ok(Json(LoginResponse {
        token: token.as_str().to_string(),
        message: format!("Welcome, {}! Login successful.", req.username),
    })
    .into_response())
}

fn get_user_roles_and_permissions(username: &str) -> (Vec<String>, Vec<String>) {
    match username {
        "admin" => (
            vec!["admin".to_string(), "user".to_string()],
            vec![
                "user:list".to_string(),
                "user:add".to_string(),
                "user:edit".to_string(),
                "user:delete".to_string(),
            ],
        ),
        _ => (vec!["user".to_string()], vec!["user:list".to_string()]),
    }
}

// ============================================================================
// Protected routes (authentication required)
// ============================================================================

#[get("/user/info")]
async fn user_info(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    let roles = StpUtil::get_roles(&user_id).await;
    let permissions = StpUtil::get_permissions(&user_id).await;

    Ok(Json(UserInfo {
        user_id: user_id.clone(),
        roles,
        permissions,
        message: format!("Hello, {}! You are authenticated.", user_id),
    }))
}

#[get("/user/token-info")]
async fn token_info(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    let token = StpUtil::get_token_by_login_id(&user_id)
        .await
        .map(|t| t.as_str().to_string())
        .unwrap_or_default();

    let is_login = StpUtil::is_login_by_login_id(&user_id).await;

    Ok(Json(TokenInfo {
        token,
        login_id: user_id,
        is_login,
    }))
}

#[post("/user/logout")]
async fn logout(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    StpUtil::logout_by_login_id(&user_id)
        .await
        .context("Logout failed")?;

    Ok(Json(MessageResponse {
        message: format!("Goodbye, {}! You have been logged out.", user_id),
    }))
}

#[get("/user/check-permission/{permission}")]
async fn check_permission(
    LoginIdExtractor(user_id): LoginIdExtractor,
    summer_web::extractor::Path(permission): summer_web::extractor::Path<String>,
) -> Result<impl IntoResponse> {
    let has_permission = StpUtil::has_permission(&user_id, &permission).await;

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "permission": permission,
        "has_permission": has_permission
    })))
}

#[get("/user/check-role/{role}")]
async fn check_role(
    LoginIdExtractor(user_id): LoginIdExtractor,
    summer_web::extractor::Path(role): summer_web::extractor::Path<String>,
) -> Result<impl IntoResponse> {
    let has_role = StpUtil::has_role(&user_id, &role).await;

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "role": role,
        "has_role": has_role
    })))
}

// ============================================================================
// Admin routes - using procedural macros for permission checking
// ============================================================================

#[get("/admin/dashboard")]
#[sa_check_role("admin")]
async fn admin_dashboard(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    Ok(Json(MessageResponse {
        message: format!("Welcome to admin dashboard, {}!", user_id),
    }))
}

#[get("/admin/users")]
#[sa_check_permission("user:list")]
async fn list_users(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    let users = vec![
        serde_json::json!({"id": 1, "name": "admin", "role": "admin"}),
        serde_json::json!({"id": 2, "name": "user1", "role": "user"}),
        serde_json::json!({"id": 3, "name": "user2", "role": "user"}),
    ];

    Ok(Json(serde_json::json!({
        "operator": user_id,
        "users": users,
        "total": 3
    })))
}

#[sa_check_permission("user:delete")]
#[post("/admin/delete")]
async fn delete_user(
    LoginIdExtractor(user_id): LoginIdExtractor,
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse> {
    let target_user = body
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(Json(MessageResponse {
        message: format!("User '{}' deleted by operator '{}'", target_user, user_id),
    }))
}

// ============================================================================
// Advanced macro examples
// ============================================================================

#[get("/api/profile")]
#[sa_check_login]
async fn get_profile(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "message": "You are logged in!"
    })))
}

#[get("/api/super-admin")]
#[sa_check_roles_and("admin", "user")]
async fn super_admin_only(
    LoginIdExtractor(user_id): LoginIdExtractor,
) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "message": "You have both admin and user roles!"
    })))
}

#[get("/api/management")]
#[sa_check_roles_or("admin", "manager")]
async fn management_area(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "message": "You have admin or manager role!"
    })))
}

#[post("/api/user/batch-modify")]
#[sa_check_permissions_and("user:edit", "user:delete")]
async fn batch_modify_users(
    LoginIdExtractor(user_id): LoginIdExtractor,
) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "message": "You have both edit and delete permissions!"
    })))
}

#[post("/api/user/create-or-update")]
#[sa_check_permissions_or("user:add", "user:edit")]
async fn create_or_update_user(
    LoginIdExtractor(user_id): LoginIdExtractor,
) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "message": "You have add or edit permission!"
    })))
}

#[get("/api/health")]
#[sa_ignore]
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "message": "This endpoint ignores authentication"
    }))
}

/// Get Sa-Token configuration - for debugging/verification
#[get("/api/config")]
#[sa_ignore]
async fn get_config(Component(state): Component<SaTokenState>) -> impl IntoResponse {
    let config = &state.manager.config;
    Json(serde_json::json!({
        "token_name": config.token_name,
        "timeout": config.timeout,
        "active_timeout": config.active_timeout,
        "auto_renew": config.auto_renew,
        "is_concurrent": config.is_concurrent,
        "is_share": config.is_share,
        "token_style": format!("{:?}", config.token_style),
        "is_log": config.is_log,
        "is_read_cookie": config.is_read_cookie,
        "is_read_header": config.is_read_header,
        "is_read_body": config.is_read_body,
        "token_prefix": config.token_prefix,
        "jwt_secret_key": config.jwt_secret_key.as_ref().map(|_| "***"),
        "jwt_algorithm": config.jwt_algorithm,
        "jwt_issuer": config.jwt_issuer,
        "jwt_audience": config.jwt_audience,
        "enable_nonce": config.enable_nonce,
        "nonce_timeout": config.nonce_timeout,
        "enable_refresh_token": config.enable_refresh_token,
        "refresh_token_timeout": config.refresh_token_timeout,
    }))
}
