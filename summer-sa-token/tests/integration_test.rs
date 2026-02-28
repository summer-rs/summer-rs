//! Integration tests for summer-sa-token middleware and extractors
//!
//! Uses AppBuilder and Plugin pattern for proper initialization

use sa_token_core::token::TokenValue;
use summer::app::AppBuilder;
use summer::async_trait;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry, Plugin};
use summer_sa_token::{OptionalSaTokenExtractor, SaTokenLayer, SaTokenState, StpUtil};
use summer_web::axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceExt;

/// Test plugin that initializes SaTokenState for testing
struct TestSaTokenPlugin;

#[async_trait]
impl Plugin for TestSaTokenPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let state = SaTokenState::builder()
            .storage(std::sync::Arc::new(summer_sa_token::MemoryStorage::new()))
            .token_name("Authorization".to_string())
            .timeout(3600)
            .build();

        app.add_component(state);
    }

    fn name(&self) -> &str {
        "TestSaTokenPlugin"
    }
}

/// Combined integration test for all SaTokenState functionality
#[tokio::test]
async fn test_sa_token_integration() {
    // Initialize app with test plugin
    let mut app = AppBuilder::default();
    TestSaTokenPlugin.build(&mut app).await;

    // Get state from app
    let state = app
        .get_component::<SaTokenState>()
        .expect("SaTokenState should be registered");

    // =========================================================================
    // Test 1: Layer creation
    // =========================================================================
    {
        let _layer = SaTokenLayer::new(state.clone());
    }

    // =========================================================================
    // Test 2: Login and token validation
    // =========================================================================
    {
        let token = state.manager.login("test_user").await.unwrap();
        assert!(!token.as_str().is_empty());

        let is_valid = state.manager.is_valid(&token).await;
        assert!(is_valid);

        let token_info = state.manager.get_token_info(&token).await;
        assert!(token_info.is_ok());
        assert_eq!(token_info.unwrap().login_id, "test_user");
    }

    // =========================================================================
    // Test 3: Logout
    // =========================================================================
    {
        let token = state.manager.login("logout_user").await.unwrap();
        assert!(state.manager.is_valid(&token).await);

        state
            .manager
            .logout_by_login_id("logout_user")
            .await
            .unwrap();
        assert!(!state.manager.is_valid(&token).await);
    }

    // =========================================================================
    // Test 4: Multiple tokens same user
    // =========================================================================
    {
        let token1 = state.manager.login("multi_user").await.unwrap();
        let token2 = state.manager.login("multi_user").await.unwrap();

        assert!(state.manager.is_valid(&token1).await);
        assert!(state.manager.is_valid(&token2).await);
    }

    // =========================================================================
    // Test 5: Token info
    // =========================================================================
    {
        let token = state.manager.login("info_user").await.unwrap();
        let info = state.manager.get_token_info(&token).await.unwrap();
        assert_eq!(info.login_id, "info_user");
    }

    // =========================================================================
    // Test 6: Invalid token
    // =========================================================================
    {
        let fake_token = TokenValue::new("fake-token-12345");
        assert!(!state.manager.is_valid(&fake_token).await);
    }

    // =========================================================================
    // Test 7: Roles and permissions
    // =========================================================================
    {
        let _token = state.manager.login("role_user").await.unwrap();

        StpUtil::set_roles("role_user", vec!["admin".to_string(), "user".to_string()])
            .await
            .unwrap();

        StpUtil::set_permissions(
            "role_user",
            vec!["user:read".to_string(), "user:write".to_string()],
        )
        .await
        .unwrap();

        assert!(StpUtil::has_role("role_user", "admin").await);
        assert!(StpUtil::has_role("role_user", "user").await);
        assert!(!StpUtil::has_role("role_user", "superadmin").await);

        assert!(StpUtil::has_permission("role_user", "user:read").await);
        assert!(StpUtil::has_permission("role_user", "user:write").await);
        assert!(!StpUtil::has_permission("role_user", "user:delete").await);

        let roles = StpUtil::get_roles("role_user").await;
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&"admin".to_string()));

        let permissions = StpUtil::get_permissions("role_user").await;
        assert_eq!(permissions.len(), 2);
        assert!(permissions.contains(&"user:read".to_string()));
    }

    // =========================================================================
    // Test 8: Middleware with valid token
    // =========================================================================
    {
        let token = state.manager.login("middleware_user").await.unwrap();

        async fn handler(optional_token: OptionalSaTokenExtractor) -> impl IntoResponse {
            match optional_token.0 {
                Some(token) => format!("Token: {}", token.as_str()),
                None => "No token".to_string(),
            }
        }

        let app = Router::new()
            .route("/test", get(handler))
            .layer(SaTokenLayer::new(state.clone()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("Authorization", token.as_str())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // =========================================================================
    // Test 9: Middleware without token
    // =========================================================================
    {
        async fn handler(optional_token: OptionalSaTokenExtractor) -> impl IntoResponse {
            match optional_token.0 {
                Some(_) => "Has token",
                None => "No token",
            }
        }

        let app = Router::new()
            .route("/test", get(handler))
            .layer(SaTokenLayer::new(state.clone()));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

mod test_config_conversion {
    use summer_sa_token::{CoreConfig, SaTokenConfig};

    #[test]
    fn test_config_into_core_config() {
        let config = SaTokenConfig {
            token_name: "TestToken".to_string(),
            timeout: 7200,
            auto_renew: true,
            ..Default::default()
        };

        let core_config: CoreConfig = config.into();
        assert_eq!(core_config.token_name, "TestToken");
        assert_eq!(core_config.timeout, 7200);
        assert!(core_config.auto_renew);
    }
}

mod test_path_auth_builder {
    use summer_sa_token::PathAuthBuilder;

    #[test]
    fn test_path_auth_builder_creation() {
        let builder = PathAuthBuilder::new();
        assert!(!builder.is_configured());
    }

    #[test]
    fn test_path_auth_builder_include() {
        let builder = PathAuthBuilder::new()
            .include("/api/**")
            .include("/admin/**");

        assert!(builder.is_configured());
    }

    #[test]
    fn test_path_auth_builder_exclude() {
        let builder = PathAuthBuilder::new()
            .include("/api/**")
            .exclude("/api/public/**")
            .exclude("/login");

        assert!(builder.is_configured());
    }

    #[test]
    fn test_path_auth_builder_include_all() {
        let builder = PathAuthBuilder::new().include_all(["/api/**", "/admin/**", "/user/**"]);

        assert!(builder.is_configured());
    }

    #[test]
    fn test_path_auth_builder_exclude_all() {
        let builder = PathAuthBuilder::new()
            .include("/api/**")
            .exclude_all(["/api/public/**", "/api/health"]);

        assert!(builder.is_configured());
    }

    #[test]
    fn test_path_auth_builder_aliases() {
        let builder = PathAuthBuilder::new()
            .authenticated("/api/**")
            .permit_all("/api/public/**");

        assert!(builder.is_configured());
    }

    #[test]
    fn test_path_auth_builder_merge() {
        let builder1 = PathAuthBuilder::new().include("/api/**");
        let builder2 = PathAuthBuilder::new().include("/admin/**");

        let merged = builder1.merge(builder2);
        assert!(merged.is_configured());
    }

    #[test]
    fn test_path_auth_builder_build() {
        let builder = PathAuthBuilder::new()
            .include("/api/**")
            .exclude("/api/public/**");

        // PathAuthConfig is built successfully
        let _config = builder.build();
    }
}
