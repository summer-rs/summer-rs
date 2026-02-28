#![doc = include_str!("../../Plugin.md")]

/// Component definition
pub mod component;
/// Lazy component loading for circular dependencies
pub mod lazy;
/// Service is a special Component that supports dependency injection at compile time
pub mod service;

use crate::error::Result;
use crate::{app::AppBuilder, error::AppError};
use async_trait::async_trait;
use component::ComponentRef;
pub use lazy::LazyComponent;
pub use service::Service;
use std::{
    any::{self, Any},
    ops::Deref,
    sync::Arc,
};

// Define inventory collection for auto-registered plugins
inventory::collect!(&'static dyn Plugin);

/// Submit a component plugin for automatic registration
///
/// This macro wraps `inventory::submit!` to avoid users needing to
/// directly depend on the `inventory` crate.
///
/// # Example
/// ```ignore
/// submit_component_plugin!(MyComponentPlugin);
/// ```
#[macro_export]
macro_rules! submit_component_plugin {
    ($ty:ident) => {
        $crate::submit_inventory! {
            &$ty as &dyn $crate::plugin::Plugin
        }
    };
}

/// Plugin Reference
#[derive(Clone)]
pub struct PluginRef(Arc<dyn Plugin>);

/// Defined plugin interface
#[async_trait]
pub trait Plugin: Any + Send + Sync {
    /// Configures the `App` to which this plugin is added.
    async fn build(&self, _app: &mut AppBuilder) {}

    /// Configures the `App` to which this plugin is added.
    /// The immediately plugin will not be added to the registry,
    /// and the plugin cannot obtain components registered in the registry.
    fn immediately_build(&self, _app: &mut AppBuilder) {}

    /// Configures a name for the [`Plugin`] which is primarily used for checking plugin
    /// uniqueness and debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// A list of plugins to depend on. The plugin will be built after the plugins in this list.
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// Whether the plugin should be built immediately when added
    fn immediately(&self) -> bool {
        false
    }
}

impl PluginRef {
    pub(crate) fn new<T: Plugin>(plugin: T) -> Self {
        Self(Arc::new(plugin))
    }
}

impl Deref for PluginRef {
    type Target = dyn Plugin;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

/// Component Registry
pub trait ComponentRegistry {
    /// Get the component reference of the specified type
    fn get_component_ref<T>(&self) -> Option<ComponentRef<T>>
    where
        T: Any + Send + Sync;

    /// Get the component reference of the specified type.
    /// If the component does not exist, it will panic.
    fn get_expect_component_ref<T>(&self) -> ComponentRef<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.get_component_ref().unwrap_or_else(|| {
            panic!(
                "{} component not exists in registry",
                std::any::type_name::<T>()
            )
        })
    }

    /// Get the component reference of the specified type.
    /// If the component does not exist, it will return AppError::ComponentNotExist.
    fn try_get_component_ref<T>(&self) -> Result<ComponentRef<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.get_component_ref()
            .ok_or_else(|| AppError::ComponentNotExist(std::any::type_name::<T>()))
    }

    /// Get the component of the specified type
    fn get_component<T>(&self) -> Option<T>
    where
        T: Clone + Send + Sync + 'static;

    /// Get the component of the specified type.
    /// If the component does not exist, it will panic.
    fn get_expect_component<T>(&self) -> T
    where
        T: Clone + Send + Sync + 'static,
    {
        self.get_component().unwrap_or_else(|| {
            panic!(
                "{} component not exists in registry",
                std::any::type_name::<T>()
            )
        })
    }

    /// Get the component of the specified type.
    /// If the component does not exist, it will return AppError::ComponentNotExist.
    fn try_get_component<T>(&self) -> Result<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.get_component()
            .ok_or_else(|| AppError::ComponentNotExist(std::any::type_name::<T>()))
    }

    /// Is there a component of the specified type in the registry?
    fn has_component<T>(&self) -> bool
    where
        T: Any + Send + Sync;
}

/// Mutable Component Registry
pub trait MutableComponentRegistry: ComponentRegistry {
    /// Add component to the registry
    fn add_component<C>(&mut self, component: C) -> &mut Self
    where
        C: Clone + any::Any + Send + Sync;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppBuilder;

    #[derive(Clone)]
    struct TestComponent {
        value: i32,
    }

    #[derive(Clone)]
    struct AnotherComponent {
        name: String,
    }

    #[tokio::test]
    async fn test_component_registry_add_and_get() {
        let mut app = AppBuilder::default();

        let test_comp = TestComponent { value: 42 };
        app.add_component(test_comp);

        // Should be able to get the component
        let retrieved = app.get_component::<TestComponent>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, 42);
    }

    #[tokio::test]
    async fn test_component_registry_get_nonexistent() {
        let app = AppBuilder::default();

        // Should return None for non-existent component
        let retrieved = app.get_component::<TestComponent>();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_component_registry_has_component() {
        let mut app = AppBuilder::default();

        assert!(!app.has_component::<TestComponent>());

        app.add_component(TestComponent { value: 100 });

        assert!(app.has_component::<TestComponent>());
        assert!(!app.has_component::<AnotherComponent>());
    }

    #[tokio::test]
    async fn test_component_registry_multiple_types() {
        let mut app = AppBuilder::default();

        app.add_component(TestComponent { value: 1 });
        app.add_component(AnotherComponent {
            name: "test".to_string(),
        });

        // Both should be retrievable
        let test_comp = app.get_component::<TestComponent>();
        let another_comp = app.get_component::<AnotherComponent>();

        assert!(test_comp.is_some());
        assert!(another_comp.is_some());
        assert_eq!(test_comp.unwrap().value, 1);
        assert_eq!(another_comp.unwrap().name, "test");
    }

    #[tokio::test]
    async fn test_get_component_ref() {
        let mut app = AppBuilder::default();
        app.add_component(TestComponent { value: 999 });

        let comp_ref = app.get_component_ref::<TestComponent>();
        assert!(comp_ref.is_some());

        let comp_ref = comp_ref.unwrap();
        assert_eq!(comp_ref.value, 999);
    }

    #[tokio::test]
    async fn test_try_get_component_success() {
        let mut app = AppBuilder::default();
        app.add_component(TestComponent { value: 50 });

        let result = app.try_get_component::<TestComponent>();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, 50);
    }

    #[tokio::test]
    async fn test_try_get_component_failure() {
        let app = AppBuilder::default();

        let result = app.try_get_component::<TestComponent>();
        assert!(result.is_err());
    }

    #[tokio::test]
    #[should_panic(expected = "component not exists in registry")]
    async fn test_get_expect_component_panic() {
        let app = AppBuilder::default();
        let _comp = app.get_expect_component::<TestComponent>();
    }

    #[tokio::test]
    async fn test_get_expect_component_success() {
        let mut app = AppBuilder::default();
        app.add_component(TestComponent { value: 777 });

        let comp = app.get_expect_component::<TestComponent>();
        assert_eq!(comp.value, 777);
    }

    // Plugin tests
    struct TestPlugin {
        name: &'static str,
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        async fn build(&self, app: &mut AppBuilder) {
            app.add_component(TestComponent { value: 123 });
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    struct DependentPlugin;

    #[async_trait]
    impl Plugin for DependentPlugin {
        async fn build(&self, app: &mut AppBuilder) {
            // This plugin depends on TestPlugin's component
            let test_comp = app.get_component::<TestComponent>();
            assert!(test_comp.is_some());

            app.add_component(AnotherComponent {
                name: format!("dependent_{}", test_comp.unwrap().value),
            });
        }

        fn name(&self) -> &str {
            "DependentPlugin"
        }

        fn dependencies(&self) -> Vec<&str> {
            vec!["TestPlugin"]
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let mut app = AppBuilder::default();
        app.add_plugin(TestPlugin { name: "TestPlugin" });

        assert!(app.is_plugin_added::<TestPlugin>());
    }

    #[tokio::test]
    async fn test_plugin_build_adds_component() {
        let mut app = AppBuilder::default();
        app.add_plugin(TestPlugin { name: "TestPlugin" });

        // Don't call build() to avoid tracing initialization conflict
        // Instead, manually trigger plugin build
        let plugin = TestPlugin { name: "TestPlugin" };
        plugin.build(&mut app).await;

        let comp = app.get_component::<TestComponent>();
        assert!(comp.is_some());
        assert_eq!(comp.unwrap().value, 123);
    }

    #[tokio::test]
    async fn test_plugin_dependencies_order() {
        use std::sync::Once;
        static INIT: Once = Once::new();

        // Only initialize tracing once for all tests
        INIT.call_once(|| {
            let _ = tracing_subscriber::fmt().try_init();
        });

        let mut app = AppBuilder::default();

        // Add in reverse order - dependency resolution should handle this
        app.add_plugin(DependentPlugin);
        app.add_plugin(TestPlugin { name: "TestPlugin" });

        // Manually build plugins to test dependency order
        let test_plugin = TestPlugin { name: "TestPlugin" };
        test_plugin.build(&mut app).await;

        let dependent_plugin = DependentPlugin;
        dependent_plugin.build(&mut app).await;

        // Both components should exist
        assert!(app.has_component::<TestComponent>());
        assert!(app.has_component::<AnotherComponent>());

        let another = app.get_component::<AnotherComponent>().unwrap();
        assert_eq!(another.name, "dependent_123");
    }

    struct ImmediatePlugin;

    #[async_trait]
    impl Plugin for ImmediatePlugin {
        fn immediately_build(&self, app: &mut AppBuilder) {
            app.add_component(TestComponent { value: 999 });
        }

        fn immediately(&self) -> bool {
            true
        }

        fn name(&self) -> &str {
            "ImmediatePlugin"
        }
    }

    #[tokio::test]
    async fn test_immediate_plugin() {
        let mut app = AppBuilder::default();

        // Immediate plugin should build right away
        app.add_plugin(ImmediatePlugin);

        // Component should be available immediately
        assert!(app.has_component::<TestComponent>());
        let comp = app.get_component::<TestComponent>().unwrap();
        assert_eq!(comp.value, 999);
    }

    #[tokio::test]
    #[should_panic(expected = "plugin was already added")]
    async fn test_duplicate_plugin_panic() {
        let mut app = AppBuilder::default();
        app.add_plugin(TestPlugin { name: "TestPlugin" });
        app.add_plugin(TestPlugin { name: "TestPlugin" }); // Should panic
    }

    #[tokio::test]
    #[should_panic(expected = "component was already added")]
    async fn test_duplicate_component_panic() {
        let mut app = AppBuilder::default();
        app.add_component(TestComponent { value: 1 });
        app.add_component(TestComponent { value: 2 }); // Should panic
    }
}
