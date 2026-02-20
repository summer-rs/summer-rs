use spring_redis::config::RedisConfig;

#[test]
fn test_redis_config_creation() {
    let config = RedisConfig {
        uri: "redis://localhost:6379".to_string(),
        exponent_base: Some(2),
        number_of_retries: Some(3),
        max_delay: Some(5000),
        response_timeout: Some(3000),
        connection_timeout: Some(2000),
    };
    
    assert_eq!(config.uri, "redis://localhost:6379");
    assert_eq!(config.exponent_base, Some(2));
    assert_eq!(config.number_of_retries, Some(3));
}

#[test]
fn test_redis_config_default_values() {
    let config = RedisConfig {
        uri: "redis://localhost:6379".to_string(),
        exponent_base: None,
        number_of_retries: None,
        max_delay: None,
        response_timeout: None,
        connection_timeout: None,
    };
    
    assert_eq!(config.uri, "redis://localhost:6379");
    assert!(config.exponent_base.is_none());
    assert!(config.number_of_retries.is_none());
}

#[test]
fn test_redis_config_toml_deserialization() {
    let toml_str = r#"
        uri = "redis://127.0.0.1:6379"
        exponent_base = 2
        number_of_retries = 3
    "#;
    
    let config: Result<RedisConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.uri, "redis://127.0.0.1:6379");
    assert_eq!(config.exponent_base, Some(2));
}

#[test]
fn test_redis_config_minimal_toml() {
    let toml_str = r#"
        uri = "redis://localhost"
    "#;
    
    let config: Result<RedisConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.uri, "redis://localhost");
}

#[test]
fn test_redis_config_with_auth() {
    let config = RedisConfig {
        uri: "redis://:password@localhost:6379".to_string(),
        exponent_base: None,
        number_of_retries: None,
        max_delay: None,
        response_timeout: None,
        connection_timeout: None,
    };
    
    assert!(config.uri.contains("password"));
}

#[test]
fn test_redis_config_with_db_number() {
    let config = RedisConfig {
        uri: "redis://localhost:6379/2".to_string(),
        exponent_base: None,
        number_of_retries: None,
        max_delay: None,
        response_timeout: None,
        connection_timeout: None,
    };
    
    assert!(config.uri.ends_with("/2"));
}

#[test]
fn test_redis_config_clone() {
    let config = RedisConfig {
        uri: "redis://localhost:6379".to_string(),
        exponent_base: Some(2),
        number_of_retries: Some(3),
        max_delay: Some(5000),
        response_timeout: Some(3000),
        connection_timeout: Some(2000),
    };
    
    let cloned = config.clone();
    assert_eq!(config.uri, cloned.uri);
    assert_eq!(config.exponent_base, cloned.exponent_base);
}

#[test]
fn test_redis_config_timeouts() {
    let config = RedisConfig {
        uri: "redis://localhost:6379".to_string(),
        exponent_base: None,
        number_of_retries: None,
        max_delay: Some(5000),
        response_timeout: Some(3000),
        connection_timeout: Some(2000),
    };
    
    assert!(config.response_timeout.unwrap() > 0);
    assert!(config.connection_timeout.unwrap() > 0);
    assert!(config.max_delay.unwrap() > 0);
}

#[test]
fn test_redis_config_retry_settings() {
    let config = RedisConfig {
        uri: "redis://localhost:6379".to_string(),
        exponent_base: Some(2),
        number_of_retries: Some(5),
        max_delay: Some(10000),
        response_timeout: None,
        connection_timeout: None,
    };
    
    assert_eq!(config.exponent_base, Some(2));
    assert_eq!(config.number_of_retries, Some(5));
    assert_eq!(config.max_delay, Some(10000));
}

