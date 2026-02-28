use summer_sqlx::config::SqlxConfig;

#[test]
fn test_sqlx_config_creation() {
    let config = SqlxConfig {
        uri: "postgres://user:pass@localhost/mydb".to_string(),
        min_connections: 5,
        max_connections: 20,
        connect_timeout: Some(5000),
        idle_timeout: Some(600000),
        acquire_timeout: Some(30000),
    };
    
    assert_eq!(config.uri, "postgres://user:pass@localhost/mydb");
    assert_eq!(config.min_connections, 5);
    assert_eq!(config.max_connections, 20);
}

#[test]
fn test_sqlx_config_default_values() {
    let toml_str = r#"
        uri = "postgres://localhost/test"
    "#;
    
    let config: SqlxConfig = toml::from_str(toml_str).unwrap();
    
    // Check default values
    assert_eq!(config.min_connections, 1);
    assert_eq!(config.max_connections, 10);
}

#[test]
fn test_sqlx_config_postgres_uri() {
    let config = SqlxConfig {
        uri: "postgres://user:password@localhost:5432/database".to_string(),
        min_connections: 1,
        max_connections: 10,
        connect_timeout: None,
        idle_timeout: None,
        acquire_timeout: None,
    };
    
    assert!(config.uri.starts_with("postgres://"));
    assert!(config.uri.contains("localhost"));
}

#[test]
fn test_sqlx_config_mysql_uri() {
    let config = SqlxConfig {
        uri: "mysql://user:password@localhost:3306/database".to_string(),
        min_connections: 1,
        max_connections: 10,
        connect_timeout: None,
        idle_timeout: None,
        acquire_timeout: None,
    };
    
    assert!(config.uri.starts_with("mysql://"));
}

#[test]
fn test_sqlx_config_sqlite_uri() {
    let config = SqlxConfig {
        uri: "sqlite:./test.db".to_string(),
        min_connections: 1,
        max_connections: 10,
        connect_timeout: None,
        idle_timeout: None,
        acquire_timeout: None,
    };
    
    assert!(config.uri.starts_with("sqlite:"));
}

#[test]
fn test_sqlx_config_toml_deserialization() {
    let toml_str = r#"
        uri = "postgres://localhost/mydb"
        min_connections = 2
        max_connections = 15
        connect_timeout = 3000
        idle_timeout = 300000
        acquire_timeout = 20000
    "#;
    
    let config: Result<SqlxConfig, _> = toml::from_str(toml_str);
    assert!(config.is_ok());
    
    let config = config.unwrap();
    assert_eq!(config.min_connections, 2);
    assert_eq!(config.max_connections, 15);
    assert_eq!(config.connect_timeout, Some(3000));
}

#[test]
fn test_sqlx_config_connection_pool_limits() {
    let config = SqlxConfig {
        uri: "postgres://localhost/test".to_string(),
        min_connections: 5,
        max_connections: 50,
        connect_timeout: None,
        idle_timeout: None,
        acquire_timeout: None,
    };
    
    assert!(config.min_connections <= config.max_connections);
    assert!(config.min_connections > 0);
}

#[test]
fn test_sqlx_config_timeouts_optional() {
    let config = SqlxConfig {
        uri: "postgres://localhost/test".to_string(),
        min_connections: 1,
        max_connections: 10,
        connect_timeout: None,
        idle_timeout: None,
        acquire_timeout: None,
    };
    
    assert!(config.connect_timeout.is_none());
    assert!(config.idle_timeout.is_none());
    assert!(config.acquire_timeout.is_none());
}

#[test]
fn test_sqlx_config_with_all_timeouts() {
    let config = SqlxConfig {
        uri: "postgres://localhost/test".to_string(),
        min_connections: 1,
        max_connections: 10,
        connect_timeout: Some(5000),
        idle_timeout: Some(600000),
        acquire_timeout: Some(30000),
    };
    
    assert!(config.connect_timeout.is_some());
    assert!(config.idle_timeout.is_some());
    assert!(config.acquire_timeout.is_some());
}

#[test]
fn test_sqlx_config_clone() {
    let config = SqlxConfig {
        uri: "postgres://localhost/test".to_string(),
        min_connections: 3,
        max_connections: 15,
        connect_timeout: Some(3000),
        idle_timeout: Some(300000),
        acquire_timeout: Some(20000),
    };
    
    let cloned = config.clone();
    assert_eq!(config.uri, cloned.uri);
    assert_eq!(config.min_connections, cloned.min_connections);
    assert_eq!(config.max_connections, cloned.max_connections);
}

#[test]
fn test_sqlx_config_partial_toml() {
    let toml_str = r#"
        uri = "postgres://localhost/test"
        max_connections = 25
    "#;
    
    let config: SqlxConfig = toml::from_str(toml_str).unwrap();
    
    assert_eq!(config.uri, "postgres://localhost/test");
    assert_eq!(config.max_connections, 25);
    assert_eq!(config.min_connections, 1); // default
}

#[test]
fn test_sqlx_config_invalid_toml() {
    let toml_str = r#"
        # Missing required uri field
        min_connections = 1
    "#;
    
    let config: Result<SqlxConfig, _> = toml::from_str(toml_str);
    assert!(config.is_err());
}

#[test]
fn test_sqlx_config_uri_with_options() {
    let config = SqlxConfig {
        uri: "postgres://localhost/test?sslmode=require".to_string(),
        min_connections: 1,
        max_connections: 10,
        connect_timeout: None,
        idle_timeout: None,
        acquire_timeout: None,
    };
    
    assert!(config.uri.contains("sslmode=require"));
}

