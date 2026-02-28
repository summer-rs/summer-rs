//! Configuration tests for summer-sa-token

use summer_sa_token::{SaTokenConfig, TokenStyle};

#[test]
fn test_sa_token_config_creation() {
    let config = SaTokenConfig {
        token_name: "Authorization".to_string(),
        timeout: 86400,
        active_timeout: -1,
        auto_renew: true,
        is_concurrent: true,
        is_share: true,
        token_style: TokenStyle::Uuid,
        is_log: false,
        is_read_cookie: true,
        is_read_header: true,
        is_read_body: false,
        token_prefix: Some("Bearer ".to_string()),
        jwt_secret_key: None,
        jwt_algorithm: Some("HS256".to_string()),
        jwt_issuer: None,
        jwt_audience: None,
        enable_nonce: false,
        nonce_timeout: -1,
        enable_refresh_token: false,
        refresh_token_timeout: 604800,
    };

    assert_eq!(config.token_name, "Authorization");
    assert_eq!(config.timeout, 86400);
    assert!(config.auto_renew);
    assert!(config.is_concurrent);
}

#[test]
fn test_sa_token_config_default() {
    let config = SaTokenConfig::default();

    assert_eq!(config.token_name, "Authorization");
    assert_eq!(config.timeout, 2592000); // 30 days
    assert_eq!(config.active_timeout, -1);
    assert!(!config.auto_renew);
    assert!(config.is_concurrent);
    assert!(config.is_share);
    assert!(matches!(config.token_style, TokenStyle::Uuid));
}

#[test]
fn test_sa_token_config_toml_deserialization() {
    let toml_str = r#"
        token_name = "Authorization"
        timeout = 3600
        auto_renew = true
        is_concurrent = false
        token_style = "Uuid"
    "#;

    let config: Result<SaTokenConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.token_name, "Authorization");
    assert_eq!(config.timeout, 3600);
    assert!(config.auto_renew);
    assert!(!config.is_concurrent);
}

#[test]
fn test_sa_token_config_minimal_toml() {
    // All fields have defaults, so even minimal config should work
    let toml_str = r#"
        token_name = "X-Token"
    "#;

    let config: Result<SaTokenConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.token_name, "X-Token");
    // Check defaults are applied
    assert_eq!(config.timeout, 2592000);
}

#[test]
fn test_sa_token_config_with_jwt() {
    let toml_str = r#"
        token_name = "Authorization"
        token_style = "Jwt"
        jwt_secret_key = "my-secret-key"
        jwt_algorithm = "HS512"
        jwt_issuer = "my-app"
    "#;

    let config: Result<SaTokenConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert!(matches!(config.token_style, TokenStyle::Jwt));
    assert_eq!(config.jwt_secret_key, Some("my-secret-key".to_string()));
    assert_eq!(config.jwt_algorithm, Some("HS512".to_string()));
    assert_eq!(config.jwt_issuer, Some("my-app".to_string()));
}

#[test]
fn test_sa_token_config_with_prefix() {
    let config = SaTokenConfig {
        token_prefix: Some("Bearer ".to_string()),
        ..Default::default()
    };

    assert_eq!(config.token_prefix, Some("Bearer ".to_string()));
}

#[test]
fn test_sa_token_config_clone() {
    let config = SaTokenConfig {
        token_name: "TestToken".to_string(),
        timeout: 7200,
        ..Default::default()
    };

    let cloned = config.clone();
    assert_eq!(config.token_name, cloned.token_name);
    assert_eq!(config.timeout, cloned.timeout);
}

#[test]
fn test_token_style_variants() {
    let toml_uuid = r#"token_style = "Uuid""#;
    let toml_random32 = r#"token_style = "Random32""#;
    let toml_random64 = r#"token_style = "Random64""#;
    let toml_random128 = r#"token_style = "Random128""#;
    let toml_jwt = r#"token_style = "Jwt""#;

    #[derive(serde::Deserialize)]
    struct StyleOnly {
        token_style: TokenStyle,
    }

    assert!(matches!(
        toml::from_str::<StyleOnly>(toml_uuid).unwrap().token_style,
        TokenStyle::Uuid
    ));
    assert!(matches!(
        toml::from_str::<StyleOnly>(toml_random32)
            .unwrap()
            .token_style,
        TokenStyle::Random32
    ));
    assert!(matches!(
        toml::from_str::<StyleOnly>(toml_random64)
            .unwrap()
            .token_style,
        TokenStyle::Random64
    ));
    assert!(matches!(
        toml::from_str::<StyleOnly>(toml_random128)
            .unwrap()
            .token_style,
        TokenStyle::Random128
    ));
    assert!(matches!(
        toml::from_str::<StyleOnly>(toml_jwt).unwrap().token_style,
        TokenStyle::Jwt
    ));
}

#[test]
fn test_sa_token_config_refresh_token() {
    let toml_str = r#"
        token_name = "Authorization"
        enable_refresh_token = true
        refresh_token_timeout = 1209600
    "#;

    let config: Result<SaTokenConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert!(config.enable_refresh_token);
    assert_eq!(config.refresh_token_timeout, 1209600); // 14 days
}

#[test]
fn test_sa_token_config_nonce() {
    let toml_str = r#"
        token_name = "Authorization"
        enable_nonce = true
        nonce_timeout = 300
    "#;

    let config: Result<SaTokenConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert!(config.enable_nonce);
    assert_eq!(config.nonce_timeout, 300);
}