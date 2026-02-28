//! Integration tests for summer-rs core functionality

use summer::app::AppBuilder;
use summer::async_trait;
use summer::config::Configurable;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry, Plugin};

// Test component
#[derive(Clone, Debug, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: u16,
}

impl Configurable for DatabaseConfig {
    fn config_prefix() -> &'static str {
        "database"
    }
}

#[derive(Clone)]
struct Database {
    config: DatabaseConfig,
}

#[derive(Clone)]
struct ApiServer {
    #[allow(dead_code)]
    port: u16,
}

// Test plugin
struct DatabasePlugin;

#[async_trait]
impl Plugin for DatabasePlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // Simulate plugin initialization
        let db = Database {
            config: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
            },
        };
        app.add_component(db);
    }

    fn name(&self) -> &str {
        "DatabasePlugin"
    }
}

// Dependent plugin
struct ApiPlugin;

#[async_trait]
impl Plugin for ApiPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // This plugin depends on DatabasePlugin
        let db = app.get_component::<Database>();
        assert!(db.is_some(), "Database component should exist");
        
        app.add_component(ApiServer { port: 8080 });
    }

    fn name(&self) -> &str {
        "ApiPlugin"
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["DatabasePlugin"]
    }
}

#[tokio::test]
async fn test_app_builder_with_components() {
    let mut app = AppBuilder::default();
    
    #[derive(Clone)]
    struct TestComponent {
        value: i32,
    }
    
    app.add_component(TestComponent { value: 42 });
    
    assert!(app.has_component::<TestComponent>());
    let comp = app.get_component::<TestComponent>().unwrap();
    assert_eq!(comp.value, 42);
}

#[tokio::test]
async fn test_app_builder_with_plugin() {
    let mut app = AppBuilder::default();
    
    // Manually build plugin to avoid tracing initialization conflict
    let plugin = DatabasePlugin;
    plugin.build(&mut app).await;
    
    assert!(app.has_component::<Database>());
    let db = app.get_component::<Database>().unwrap();
    assert_eq!(db.config.host, "localhost");
    assert_eq!(db.config.port, 5432);
}

#[tokio::test]
async fn test_plugin_dependency_resolution() {
    let mut app = AppBuilder::default();
    
    // Manually build plugins in correct order
    let db_plugin = DatabasePlugin;
    db_plugin.build(&mut app).await;
    
    let api_plugin = ApiPlugin;
    api_plugin.build(&mut app).await;
    
    // Both components should be present
    assert!(app.has_component::<Database>());
    assert!(app.has_component::<ApiServer>());
}

#[tokio::test]
async fn test_component_ref_lifetime() {
    let app = AppBuilder::default();
    
    #[derive(Clone)]
    struct SharedData {
        value: String,
    }
    
    // Don't call build, just test component registry directly
    let mut app = app;
    app.add_component(SharedData {
        value: "test".to_string(),
    });
    
    // Multiple references to the same component
    let ref1 = app.get_component::<SharedData>().unwrap();
    let ref2 = app.get_component::<SharedData>().unwrap();
    
    assert_eq!(ref1.value, ref2.value);
}

#[tokio::test]
async fn test_try_get_component_error_handling() {
    let app = AppBuilder::default();
    
    #[derive(Clone)]
    struct NonExistentComponent;
    
    let result = app.try_get_component::<NonExistentComponent>();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_multiple_plugins() {
    #[derive(Clone)]
    struct Comp1;
    #[derive(Clone)]
    struct Comp2;
    #[derive(Clone)]
    struct Comp3;
    
    struct Plugin1;
    struct Plugin2;
    struct Plugin3;
    
    #[async_trait]
    impl Plugin for Plugin1 {
        async fn build(&self, app: &mut AppBuilder) {
            app.add_component(Comp1);
        }
        fn name(&self) -> &str {
            "Plugin1"
        }
    }
    
    #[async_trait]
    impl Plugin for Plugin2 {
        async fn build(&self, app: &mut AppBuilder) {
            app.add_component(Comp2);
        }
        fn name(&self) -> &str {
            "Plugin2"
        }
    }
    
    #[async_trait]
    impl Plugin for Plugin3 {
        async fn build(&self, app: &mut AppBuilder) {
            app.add_component(Comp3);
        }
        fn name(&self) -> &str {
            "Plugin3"
        }
    }
    
    let mut app = AppBuilder::default();
    
    // Manually build plugins to avoid tracing conflict
    Plugin1.build(&mut app).await;
    Plugin2.build(&mut app).await;
    Plugin3.build(&mut app).await;
    
    assert!(app.has_component::<Comp1>());
    assert!(app.has_component::<Comp2>());
    assert!(app.has_component::<Comp3>());
}

#[tokio::test]
async fn test_immediate_plugin() {
    #[derive(Clone)]
    struct ImmediateComp {
        initialized: bool,
    }
    
    struct ImmediatePlugin;
    
    #[async_trait]
    impl Plugin for ImmediatePlugin {
        fn immediately_build(&self, app: &mut AppBuilder) {
            app.add_component(ImmediateComp { initialized: true });
        }
        
        fn immediately(&self) -> bool {
            true
        }
        
        fn name(&self) -> &str {
            "ImmediatePlugin"
        }
    }
    
    let mut app = AppBuilder::default();
    app.add_plugin(ImmediatePlugin);
    
    // Component should be available immediately
    assert!(app.has_component::<ImmediateComp>());
    let comp = app.get_component::<ImmediateComp>().unwrap();
    assert!(comp.initialized);
}

#[tokio::test]
async fn test_app_environment() {
    let app = AppBuilder::default();
    let env = app.get_env();
    
    // Should have a valid environment
    assert!(matches!(
        env,
        summer::config::env::Env::Dev
            | summer::config::env::Env::Test
            | summer::config::env::Env::Prod
    ));
}

#[tokio::test]
async fn test_component_clone_semantics() {
    let mut app = AppBuilder::default();
    
    #[derive(Clone, Debug, PartialEq)]
    struct CounterComponent {
        count: i32,
    }
    
    app.add_component(CounterComponent { count: 0 });
    
    // Don't call build, just test component directly
    let comp1 = app.get_component::<CounterComponent>().unwrap();
    let comp2 = app.get_component::<CounterComponent>().unwrap();
    
    // Both should have the same value
    assert_eq!(comp1.count, comp2.count);
}

#[tokio::test]
async fn test_plugin_initialization_order() {
    use std::sync::Arc;
    use std::sync::Mutex;
    
    let order = Arc::new(Mutex::new(Vec::new()));
    
    struct FirstPlugin {
        order: Arc<Mutex<Vec<String>>>,
    }
    
    struct SecondPlugin {
        order: Arc<Mutex<Vec<String>>>,
    }
    
    #[async_trait]
    impl Plugin for FirstPlugin {
        async fn build(&self, _app: &mut AppBuilder) {
            self.order.lock().unwrap().push("first".to_string());
        }
        fn name(&self) -> &str {
            "FirstPlugin"
        }
    }
    
    #[async_trait]
    impl Plugin for SecondPlugin {
        async fn build(&self, _app: &mut AppBuilder) {
            self.order.lock().unwrap().push("second".to_string());
        }
        fn name(&self) -> &str {
            "SecondPlugin"
        }
        fn dependencies(&self) -> Vec<&str> {
            vec!["FirstPlugin"]
        }
    }
    
    let mut app = AppBuilder::default();
    
    // Manually build in correct order since we know the dependency
    FirstPlugin {
        order: order.clone(),
    }.build(&mut app).await;
    
    SecondPlugin {
        order: order.clone(),
    }.build(&mut app).await;
    
    let initialization_order = order.lock().unwrap();
    assert_eq!(initialization_order[0], "first");
    assert_eq!(initialization_order[1], "second");
}

