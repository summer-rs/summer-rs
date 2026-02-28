//! Integration tests for #[component] macro

use serde::Deserialize;
use summer::app::AppBuilder;
use summer::component;
use summer::config::Configurable;
use summer::extractor::{Component, Config};
use summer::plugin::ComponentRegistry;
use summer::App;

// Test configuration
#[derive(Debug, Clone, Configurable, Deserialize)]
#[config_prefix = "test-db"]
struct TestDbConfig {
    host: String,
    port: u16,
}

// Test components
#[derive(Clone, Debug, PartialEq)]
struct TestConnection {
    url: String,
}

#[derive(Clone, Debug)]
struct TestRepository {
    conn: TestConnection,
}

#[derive(Clone, Debug)]
struct TestService {
    repo: TestRepository,
}

#[derive(Clone, Debug)]
struct AsyncConnection {
    url: String,
}

#[derive(Clone, Debug)]
struct ResultConnection {
    url: String,
}

// Component functions using #[component] macro
#[component]
fn create_test_connection(Config(config): Config<TestDbConfig>) -> TestConnection {
    TestConnection {
        url: format!("{}:{}", config.host, config.port),
    }
}

#[component]
fn create_test_repository(Component(conn): Component<TestConnection>) -> TestRepository {
    TestRepository { conn }
}

#[component]
fn create_test_service(Component(repo): Component<TestRepository>) -> TestService {
    TestService { repo }
}

// Async component
#[component]
async fn create_async_connection(Config(config): Config<TestDbConfig>) -> AsyncConnection {
    // Simulate async operation
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    AsyncConnection {
        url: format!("async-{}:{}", config.host, config.port),
    }
}

// Result return type
#[component]
fn create_connection_with_result(
    Config(config): Config<TestDbConfig>,
) -> Result<ResultConnection, String> {
    Ok(ResultConnection {
        url: format!("result-{}:{}", config.host, config.port),
    })
}

#[tokio::test]
async fn test_component_macro_basic() {
    let toml_config = r#"
        [test-db]
        host = "localhost"
        port = 5432
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // Verify all components are registered
    assert!(app.has_component::<TestConnection>());
    assert!(app.has_component::<TestRepository>());
    assert!(app.has_component::<TestService>());

    // Verify component values
    let conn = app.get_component::<TestConnection>().unwrap();
    assert_eq!(conn.url, "localhost:5432");

    let repo = app.get_component::<TestRepository>().unwrap();
    assert_eq!(repo.conn.url, "localhost:5432");

    let service = app.get_component::<TestService>().unwrap();
    assert_eq!(service.repo.conn.url, "localhost:5432");
}

#[tokio::test]
async fn test_component_dependency_order() {
    let toml_config = r#"
        [test-db]
        host = "testhost"
        port = 3306
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // All components should be available
    let service = app.get_component::<TestService>().unwrap();

    // Verify the dependency chain
    assert_eq!(service.repo.conn.url, "testhost:3306");
}

#[tokio::test]
async fn test_async_component() {
    let toml_config = r#"
        [test-db]
        host = "asynchost"
        port = 8080
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // Async component should be registered
    let conn = app.get_component::<AsyncConnection>().unwrap();
    assert_eq!(conn.url, "async-asynchost:8080");
}

#[tokio::test]
async fn test_result_return_type() {
    let toml_config = r#"
        [test-db]
        host = "resulthost"
        port = 9000
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // Component with Result return type should be registered
    let conn = app.get_component::<ResultConnection>().unwrap();
    assert_eq!(conn.url, "result-resulthost:9000");
}

#[tokio::test]
async fn test_component_clone_semantics() {
    let toml_config = r#"
        [test-db]
        host = "clonehost"
        port = 7000
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // Get the same component multiple times
    let conn1 = app.get_component::<TestConnection>().unwrap();
    let conn2 = app.get_component::<TestConnection>().unwrap();

    // Both should have the same value
    assert_eq!(conn1.url, conn2.url);
}

#[tokio::test]
async fn test_mixed_manual_and_auto_plugins() {
    use summer::async_trait;
    use summer::plugin::{MutableComponentRegistry, Plugin};

    #[derive(Clone)]
    struct ManualComponent {
        value: i32,
    }

    struct ManualPlugin;

    #[async_trait]
    impl Plugin for ManualPlugin {
        async fn build(&self, app: &mut AppBuilder) {
            app.add_component(ManualComponent { value: 999 });
        }

        fn name(&self) -> &str {
            "ManualPlugin"
        }
    }

    let toml_config = r#"
        [test-db]
        host = "mixedhost"
        port = 6000
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .add_plugin(ManualPlugin) // Manual plugin
        .build()
        .await
        .expect("Failed to build app");

    // Both manual and auto components should be available
    assert!(app.has_component::<ManualComponent>());
    assert!(app.has_component::<TestConnection>());

    let manual = app.get_component::<ManualComponent>().unwrap();
    assert_eq!(manual.value, 999);

    let conn = app.get_component::<TestConnection>().unwrap();
    assert_eq!(conn.url, "mixedhost:6000");
}

#[tokio::test]
async fn test_component_with_config_only() {
    #[derive(Clone)]
    struct ConfigOnlyComponent {
        setting: String,
    }

    #[component]
    fn create_config_only(Config(config): Config<TestDbConfig>) -> ConfigOnlyComponent {
        ConfigOnlyComponent {
            setting: config.host,
        }
    }

    let toml_config = r#"
        [test-db]
        host = "configonly"
        port = 5555
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    let comp = app.get_component::<ConfigOnlyComponent>().unwrap();
    assert_eq!(comp.setting, "configonly");
}

#[tokio::test]
async fn test_component_ref_usage() {
    let toml_config = r#"
        [test-db]
        host = "refhost"
        port = 4444
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // Test get_component_ref
    let conn_ref = app.get_component_ref::<TestConnection>();
    assert!(conn_ref.is_some());

    let conn_ref = conn_ref.unwrap();
    assert_eq!(conn_ref.url, "refhost:4444");
}

#[tokio::test]
async fn test_try_get_component() {
    let toml_config = r#"
        [test-db]
        host = "tryhost"
        port = 3333
    "#;

    let app = App::new()
        .use_config_str(toml_config)
        .build()
        .await
        .expect("Failed to build app");

    // Existing component
    let result = app.try_get_component::<TestConnection>();
    assert!(result.is_ok());

    // Non-existent component
    #[derive(Clone)]
    struct NonExistent;

    let result = app.try_get_component::<NonExistent>();
    assert!(result.is_err());
}
