use crate::banner;
use crate::config::env::Env;
use crate::config::toml::TomlConfigRegistry;
use crate::config::ConfigRegistry;
use crate::log::{BoxLayer, LogPlugin};
use crate::plugin::component::ComponentRef;
use crate::plugin::{service, ComponentRegistry, MutableComponentRegistry, Plugin};
use crate::{
    error::Result,
    plugin::{component::DynComponentRef, PluginRef},
};
use dashmap::DashMap;
use std::sync::LazyLock;
use std::any::{Any, TypeId};
use std::str::FromStr;
use std::sync::RwLock;
use std::{collections::HashSet, future::Future, path::Path, sync::Arc};
use tracing_subscriber::Layer;

/// Wrapper for dynamically registered plugins (from inventory)
struct DynPluginWrapper(&'static dyn Plugin);

#[async_trait::async_trait]
impl Plugin for DynPluginWrapper {
    async fn build(&self, app: &mut AppBuilder) {
        self.0.build(app).await
    }

    fn immediately_build(&self, app: &mut AppBuilder) {
        self.0.immediately_build(app)
    }

    fn name(&self) -> &str {
        self.0.name()
    }

    fn dependencies(&self) -> Vec<&str> {
        self.0.dependencies()
    }

    fn immediately(&self) -> bool {
        self.0.immediately()
    }
}

type Registry<T> = DashMap<TypeId, T>;
type Scheduler<T> = dyn FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<T>> + Send>;

/// Running Applications
#[derive(Default)]
pub struct App {
    env: Env,
    /// Component
    components: Registry<DynComponentRef>,
    config: TomlConfigRegistry,
}

/// AppBuilder: Application under construction
/// The application consists of three important parts:
/// - Plugin management
/// - Component management
/// - Configuration management
pub struct AppBuilder {
    pub(crate) env: Env,
    /// Tracing Layer
    pub(crate) layers: Vec<BoxLayer>,
    /// Plugin
    pub(crate) plugin_registry: Registry<PluginRef>,
    /// Dynamic plugins from inventory (auto-registered via #[component] macro)
    dynamic_plugins: Vec<&'static dyn Plugin>,
    /// Component
    components: Registry<DynComponentRef>,
    /// Configuration read from `config_path`
    config: TomlConfigRegistry,
    /// task
    schedulers: Vec<Box<Scheduler<String>>>,
    shutdown_hooks: Vec<Box<Scheduler<String>>>,
}

impl App {
    /// Preparing to build the application
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> AppBuilder {
        AppBuilder::default()
    }

    /// Currently active environment
    /// * [Env]
    pub fn get_env(&self) -> Env {
        self.env
    }

    /// Returns an instance of the currently configured global [`App`].
    ///
    /// **NOTE**: This global App is initialized after the application is built,
    /// please use it when the app is running, don't use it during the build process,
    /// such as during the plug-in build process.
    pub fn global() -> Arc<App> {
        GLOBAL_APP
            .read()
            .expect("GLOBAL_APP RwLock poisoned")
            .clone()
    }

    fn set_global(app: Arc<App>) {
        let mut global_app = GLOBAL_APP.write().expect("GLOBAL_APP RwLock poisoned");
        *global_app = app;
    }
}

static GLOBAL_APP: LazyLock<RwLock<Arc<App>>> = LazyLock::new(|| RwLock::new(Arc::new(App::default())));

unsafe impl Send for AppBuilder {}
unsafe impl Sync for AppBuilder {}

impl AppBuilder {
    /// Currently active environment
    /// * [Env]
    #[inline]
    pub fn get_env(&self) -> Env {
        self.env
    }

    /// add plugin
    pub fn add_plugin<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        log::debug!("added plugin: {}", plugin.name());
        if plugin.immediately() {
            plugin.immediately_build(self);
            return self;
        }
        let plugin_id = TypeId::of::<T>();
        if self.plugin_registry.contains_key(&plugin_id) {
            let plugin_name = plugin.name();
            panic!("Error adding plugin {plugin_name}: plugin was already added in application")
        }
        self.plugin_registry
            .insert(plugin_id, PluginRef::new(plugin));
        self
    }

    /// Add all plugins registered via inventory (from #[component] macro)
    ///
    /// This method collects all plugins that were automatically registered
    /// using the `#[component]` macro and adds them to the application.
    ///
    /// **Note**: This method is called automatically during `build_plugins()`,
    /// users don't need to call it manually.
    fn add_auto_plugins(&mut self) {
        let plugins: Vec<_> = inventory::iter::<&dyn Plugin>.into_iter().collect();
        log::debug!("Found {} auto plugins via inventory", plugins.len());
        
        for plugin in plugins {
            log::debug!("Adding auto plugin: {}", plugin.name());
            
            // Check if already added by name
            let plugin_name = plugin.name();
            if self.dynamic_plugins.iter().any(|p| p.name() == plugin_name) {
                panic!("Error adding plugin {plugin_name}: plugin was already added in application")
            }
            
            if plugin.immediately() {
                plugin.immediately_build(self);
            } else {
                self.dynamic_plugins.push(*plugin);
            }
        }
    }

    /// Returns `true` if the [`Plugin`] has already been added.
    #[inline]
    pub fn is_plugin_added<T: Plugin>(&self) -> bool {
        self.plugin_registry.contains_key(&TypeId::of::<T>())
    }

    /// The path of the configuration file, default is `./config/app.toml`.
    /// The application automatically reads the environment configuration file
    /// in the same directory according to the `summer_ENV` environment variable,
    /// such as `./config/app-dev.toml`.
    /// The environment configuration file has a higher priority and will
    /// overwrite the configuration items of the main configuration file.
    ///
    /// For specific supported environments, see the [`Env`] enum.
    pub fn use_config_file(&mut self, config_path: &str) -> &mut Self {
        self.config = TomlConfigRegistry::new(Path::new(config_path), self.env)
            .expect("config file load failed");
        self
    }

    /// Use an existing toml string to configure the application.
    /// For example, use include_str!('app.toml') to compile the file into the program.
    ///
    /// **Note**: This configuration method only supports one configuration content and does not support multiple environments.
    pub fn use_config_str(&mut self, toml_content: &str) -> &mut Self {
        self.config =
            TomlConfigRegistry::from_str(toml_content).expect("config content parse failed");
        self
    }

    /// add [tracing_subscriber::layer]
    pub fn add_layer<L>(&mut self, layer: L) -> &mut Self
    where
        L: Layer<tracing_subscriber::Registry> + Send + Sync + 'static,
    {
        self.layers.push(Box::new(layer));
        self
    }

    /// Add a scheduled task
    pub fn add_scheduler<T>(&mut self, scheduler: T) -> &mut Self
    where
        T: FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<String>> + Send> + 'static,
    {
        self.schedulers.push(Box::new(scheduler));
        self
    }

    /// Add a shutdown hook
    pub fn add_shutdown_hook<T>(&mut self, hook: T) -> &mut Self
    where
        T: FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<String>> + Send> + 'static,
    {
        self.shutdown_hooks.push(Box::new(hook));
        self
    }

    /// The `run` method is suitable for applications that contain scheduling logic,
    /// such as web, job, and stream.
    ///
    /// * [summer-web](https://docs.rs/summer-web)
    /// * [summer-job](https://docs.rs/summer-job)
    /// * [summer-stream](https://docs.rs/summer-stream)
    pub async fn run(&mut self) {
        match self.inner_run().await {
            Err(e) => {
                log::error!("{e:?}");
            }
            _ => { /* ignore */ }
        }
    }

    async fn inner_run(&mut self) -> Result<()> {
        // 1. print banner
        banner::print_banner(self);

        // 2. build plugin
        self.build_plugins().await;

        // 3. service dependency inject
        service::auto_inject_service(self)?;

        // 4. schedule
        self.schedule().await
    }

    /// Unlike the [`Self::run`] method, the `build` method is suitable for applications that do not contain scheduling logic.
    /// This method returns the built App, and developers can implement logic such as command lines and task scheduling by themselves.
    pub async fn build(&mut self) -> Result<Arc<App>> {
        // 1. build plugin
        self.build_plugins().await;

        // 2. service dependency inject
        service::auto_inject_service(self)?;

        Ok(self.build_app())
    }

    async fn build_plugins(&mut self) {
        LogPlugin.immediately_build(self);

        // Automatically collect plugins registered via #[component] macro
        self.add_auto_plugins();

        // Collect all plugins (both static and dynamic)
        let registry = std::mem::take(&mut self.plugin_registry);
        let mut to_register: Vec<PluginRef> = registry
            .iter()
            .map(|e| e.value().to_owned())
            .collect();
        
        // Add dynamic plugins (from inventory)
        let dynamic_plugins = std::mem::take(&mut self.dynamic_plugins);
        for plugin in dynamic_plugins {
            to_register.push(PluginRef::new(DynPluginWrapper(plugin)));
        }
        
        let mut registered: HashSet<String> = HashSet::new();

        while !to_register.is_empty() {
            let mut progress = false;
            let mut next_round = vec![];

            for plugin in to_register {
                let deps = plugin.dependencies();
                if deps.iter().all(|dep| registered.contains(*dep)) {
                    plugin.build(self).await;
                    registered.insert(plugin.name().to_string());
                    log::info!("{} plugin registered", plugin.name());
                    progress = true;
                } else {
                    next_round.push(plugin);
                }
            }

            if !progress {
                panic!("Cyclic dependency detected or missing dependencies for some plugins");
            }

            to_register = next_round;
        }
        self.plugin_registry = registry;
    }

    async fn schedule(&mut self) -> Result<()> {
        let app = self.build_app();

        let schedulers = std::mem::take(&mut self.schedulers);
        let mut handles = vec![];
        for task in schedulers {
            let poll_future = task(app.clone());
            let poll_future = Box::into_pin(poll_future);
            handles.push(tokio::spawn(poll_future));
        }

        while let Some(handle) = handles.pop() {
            match handle.await? {
                Err(e) => log::error!("{e:?}"),
                Ok(msg) => log::info!("scheduled result: {msg}"),
            }
        }

        // FILO: The hooks added by the plugin built first should be executed later
        while let Some(hook) = self.shutdown_hooks.pop() {
            let result = Box::into_pin(hook(app.clone())).await?;
            log::info!("shutdown result: {result}");
        }
        Ok(())
    }

    fn build_app(&mut self) -> Arc<App> {
        let components = std::mem::take(&mut self.components);
        let config = std::mem::take(&mut self.config);
        let app = Arc::new(App {
            env: self.env,
            components,
            config,
        });
        App::set_global(app.clone());
        app
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        let env = Env::init();
        let config = TomlConfigRegistry::new(Path::new("./config/app.toml"), env)
            .expect("toml config load failed");
        Self {
            env,
            config,
            layers: Default::default(),
            plugin_registry: Default::default(),
            dynamic_plugins: Default::default(),
            components: Default::default(),
            schedulers: Default::default(),
            shutdown_hooks: Default::default(),
        }
    }
}

impl ConfigRegistry for App {
    fn get_config<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned + crate::config::Configurable,
    {
        self.config.get_config::<T>()
    }
}

impl ConfigRegistry for AppBuilder {
    fn get_config<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned + crate::config::Configurable,
    {
        self.config.get_config::<T>()
    }
}

macro_rules! impl_component_registry {
    ($ty:ident) => {
        impl ComponentRegistry for $ty {
            fn get_component_ref<T>(&self) -> Option<ComponentRef<T>>
            where
                T: Any + Send + Sync,
            {
                let component_id = TypeId::of::<T>();
                let pair = self.components.get(&component_id)?;
                let component_ref = pair.value().clone();
                component_ref.downcast::<T>()
            }

            fn get_component<T>(&self) -> Option<T>
            where
                T: Clone + Send + Sync + 'static,
            {
                let component_ref = self.get_component_ref();
                component_ref.map(|arc| T::clone(&arc))
            }

            fn has_component<T>(&self) -> bool
            where
                T: Any + Send + Sync,
            {
                let component_id = TypeId::of::<T>();
                self.components.contains_key(&component_id)
            }
        }
    };
}

impl_component_registry!(App);
impl_component_registry!(AppBuilder);

impl MutableComponentRegistry for AppBuilder {
    /// Add component to the registry
    fn add_component<C>(&mut self, component: C) -> &mut Self
    where
        C: Clone + Any + Send + Sync,
    {
        let component_id = TypeId::of::<C>();
        let component_name = std::any::type_name::<C>();
        log::debug!("added component: {component_name}");
        if self.components.contains_key(&component_id) {
            panic!("Error adding component {component_name}: component was already added in application")
        }
        self.components
            .insert(component_id, DynComponentRef::new(component));
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::plugin::{ComponentRegistry, MutableComponentRegistry};
    use crate::App;

    #[tokio::test]
    async fn test_component_registry() {
        #[derive(Clone)]
        struct UnitComponent;

        #[derive(Clone)]
        struct TupleComponent(i32, i32);

        #[derive(Clone)]
        struct StructComponent {
            x: i32,
            y: i32,
        }

        #[derive(Clone)]
        struct Point<T> {
            x: T,
            y: T,
        }

        let app = App::new()
            .add_component(UnitComponent)
            .add_component(TupleComponent(1, 2))
            .add_component(StructComponent { x: 3, y: 4 })
            .add_component(Point { x: 5i64, y: 6i64 })
            .build()
            .await;
        let app = app.expect("app build failed");

        let _ = app.get_expect_component::<UnitComponent>();
        let t = app.get_expect_component::<TupleComponent>();
        assert_eq!(t.0, 1);
        assert_eq!(t.1, 2);
        let s = app.get_expect_component::<StructComponent>();
        assert_eq!(s.x, 3);
        assert_eq!(s.y, 4);
        let p = app.get_expect_component::<Point<i64>>();
        assert_eq!(p.x, 5);
        assert_eq!(p.y, 6);

        let p = app.get_component::<Point<i32>>();
        assert!(p.is_none())
    }
}
