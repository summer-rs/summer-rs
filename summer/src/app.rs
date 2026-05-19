use crate::banner;
use crate::config::env::Env;
use crate::config::toml::TomlConfigRegistry;
use crate::config::ConfigRegistry;
use crate::event::{
    AppBuiltEvent, BuilderEventPublisher, ConfigEvent, ConfigSource, EventBus, EventPublisher,
    PluginsBuiltEvent, ServicesInjectedEvent, ShutdownEvent, ShutdownPhase,
};
use crate::log::{BoxLayer, LogPlugin};
use crate::plugin::component::ComponentRef;
use crate::plugin::{service, ComponentRegistry, MutableComponentRegistry, Plugin};
use crate::{
    error::Result,
    plugin::{component::DynComponentRef, PluginRef},
};
use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::{
    collections::HashSet,
    future::Future,
    path::Path,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tracing_subscriber::Layer;

type Registry<T> = DashMap<TypeId, T>;
type PluginRegistry = DashMap<String, PluginRef>;
type SchedulerFn<T> = dyn FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<T>> + Send> + Send;
/// A one-shot scheduler stored in [`AppBuilder`]; interior mutability makes the builder `Sync`.
struct Scheduler<T> {
    inner: Mutex<Option<Box<SchedulerFn<T>>>>,
}

impl<T> Scheduler<T> {
    fn new<F>(scheduler: F) -> Self
    where
        F: FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<T>> + Send> + Send + 'static,
    {
        Self {
            inner: Mutex::new(Some(Box::new(scheduler))),
        }
    }

    fn run(&self, app: Arc<App>) -> Box<dyn Future<Output = Result<T>> + Send> {
        let scheduler = self
            .inner
            .lock()
            .expect("scheduler lock poisoned")
            .take()
            .expect("scheduler already executed");
        scheduler(app)
    }
}

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
    /// Plugin registry keyed by [`Plugin::name`]
    pub(crate) plugin_registry: PluginRegistry,
    /// Component
    components: Registry<DynComponentRef>,
    /// Configuration read from `config_path`
    config: TomlConfigRegistry,
    /// Source used to load the current configuration
    config_source: ConfigSource,
    /// task
    schedulers: Vec<Scheduler<String>>,
    shutdown_hooks: Vec<Scheduler<String>>,
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

static GLOBAL_APP: LazyLock<RwLock<Arc<App>>> =
    LazyLock::new(|| RwLock::new(Arc::new(App::default())));


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
        let plugin_name = plugin.name().to_string();
        if self.plugin_registry.contains_key(&plugin_name) {
            panic!("Error adding plugin {plugin_name}: plugin was already added in application")
        }
        self.plugin_registry
            .insert(plugin_name, PluginRef::new(plugin));
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

            let plugin_name = plugin.name().to_string();
            if self.plugin_registry.contains_key(&plugin_name) {
                panic!("Error adding plugin {plugin_name}: plugin was already added in application")
            }

            if plugin.immediately() {
                plugin.immediately_build(self);
            } else {
                self.plugin_registry
                    .insert(plugin_name, PluginRef::new(*plugin));
            }
        }
    }

    /// Returns `true` if the [`Plugin`] has already been added.
    #[inline]
    pub fn is_plugin_added<T: Plugin>(&self) -> bool {
        let tid = TypeId::of::<T>();
        let default_name = std::any::type_name::<T>();
        self.plugin_registry.iter().any(|entry| {
            entry.value().concrete_type_id() == tid || entry.key().as_str() == default_name
        })
    }

    /// The path of the configuration file, default is `./config/app.toml`.
    /// The application automatically reads the environment configuration file
    /// in the same directory according to the `SUMMER_ENV` environment variable,
    /// such as `./config/app-dev.toml`.
    /// The environment configuration file has a higher priority and will
    /// overwrite the configuration items of the main configuration file.
    ///
    /// For specific supported environments, see the [`Env`] enum.
    pub fn use_config_file(&mut self, config_path: &str) -> &mut Self {
        self.config = TomlConfigRegistry::new(Path::new(config_path), self.env)
            .expect("config file load failed");
        self.config_source = ConfigSource::File(PathBuf::from(config_path));
        self
    }

    /// Use an existing toml string to configure the application.
    /// For example, use include_str!('app.toml') to compile the file into the program.
    ///
    /// **Note**: This configuration method only supports one configuration content and does not support multiple environments.
    pub fn use_config_str(&mut self, toml_content: &str) -> &mut Self {
        self.config =
            TomlConfigRegistry::from_str(toml_content).expect("config content parse failed");
        self.config_source = ConfigSource::Inline;
        self
    }

    /// Merges a TOML document into the application configuration.
    pub fn merge_config_str(&mut self, toml: &str) -> Result<()> {
        self.config.merge_str(toml)
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
        T: FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<String>> + Send> + Send + 'static,
    {
        self.schedulers.push(Scheduler::new(scheduler));
        self
    }

    /// Add a shutdown hook
    pub fn add_shutdown_hook<T>(&mut self, hook: T) -> &mut Self
    where
        T: FnOnce(Arc<App>) -> Box<dyn Future<Output = Result<String>> + Send> + Send + 'static,
    {
        self.shutdown_hooks.push(Scheduler::new(hook));
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
        self.build_plugins().await?;

        // 3. service dependency inject
        service::auto_inject_service(self)?;
        self.publish(ServicesInjectedEvent).await?;

        // 4. schedule
        self.schedule().await
    }

    /// Unlike the [`Self::run`] method, the `build` method is suitable for applications that do not contain scheduling logic.
    /// This method returns the built App, and developers can implement logic such as command lines and task scheduling by themselves.
    pub async fn build(&mut self) -> Result<Arc<App>> {
        // 1. build plugin
        self.build_plugins().await?;

        // 2. service dependency inject
        service::auto_inject_service(self)?;
        self.publish(ServicesInjectedEvent).await?;

        self.build_app().await
    }

    async fn build_plugins(&mut self) -> Result<()> {
        // Register inventory plugins first so ConfigEvent subscribers see the full plugin set.
        self.add_auto_plugins();

        self.publish(ConfigEvent {
            env: self.env,
            source: self.config_source.clone(),
        })
        .await?;
        LogPlugin.immediately_build(self);

        // Collect all plugins
        let registry = std::mem::take(&mut self.plugin_registry);
        let mut to_register: Vec<PluginRef> =
            registry.iter().map(|e| e.value().to_owned()).collect();

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
        self.publish(PluginsBuiltEvent).await?;
        Ok(())
    }

    async fn schedule(&mut self) -> Result<()> {
        let app = self.build_app().await?;

        let schedulers = std::mem::take(&mut self.schedulers);
        let mut handles = vec![];
        for task in schedulers {
            let poll_future = task.run(app.clone());
            let poll_future = Box::into_pin(poll_future);
            handles.push(tokio::spawn(poll_future));
        }

        while let Some(handle) = handles.pop() {
            match handle.await? {
                Err(e) => log::error!("{e:?}"),
                Ok(msg) => log::info!("scheduled result: {msg}"),
            }
        }

        app.publish(ShutdownEvent {
            phase: ShutdownPhase::Starting,
        })
        .await?;

        // FILO: The hooks added by the plugin built first should be executed later
        while let Some(hook) = self.shutdown_hooks.pop() {
            let result = Box::into_pin(hook.run(app.clone())).await?;
            log::info!("shutdown result: {result}");
        }
        app.publish(ShutdownEvent {
            phase: ShutdownPhase::Completed,
        })
        .await?;
        Ok(())
    }

    async fn build_app(&mut self) -> Result<Arc<App>> {
        let components = std::mem::take(&mut self.components);
        let config = std::mem::take(&mut self.config);
        let app = Arc::new(App {
            env: self.env,
            components,
            config,
        });
        App::set_global(app.clone());
        app.publish(AppBuiltEvent { app: app.clone() }).await?;
        Ok(app)
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        let env = Env::init();
        let config = TomlConfigRegistry::new(Path::new("./config/app.toml"), env)
            .expect("toml config load failed");
        let mut builder = Self {
            env,
            config,
            config_source: ConfigSource::File(PathBuf::from("./config/app.toml")),
            layers: Default::default(),
            plugin_registry: Default::default(),
            components: Default::default(),
            schedulers: Default::default(),
            shutdown_hooks: Default::default(),
        };
        builder.add_component(EventBus::default());
        builder
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
    use crate::plugin::Plugin;
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

    struct LifecycleListenerPlugin {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    #[crate::async_trait]
    impl Plugin for LifecycleListenerPlugin {
        async fn build(&self, app: &mut super::AppBuilder) {
            use crate::event::{EventSubscriber, PluginsBuiltEvent};
            use std::sync::atomic::Ordering;

            let calls = self.calls.clone();
            app.listen(move |_: PluginsBuiltEvent, _app: &mut super::AppBuilder| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            });
        }
    }

    #[tokio::test]
    async fn test_plugin_can_subscribe_to_lifecycle_events() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        let calls = Arc::new(AtomicUsize::new(0));
        let mut app = App::new();
        app.add_plugin(LifecycleListenerPlugin {
            calls: calls.clone(),
        });

        app.build().await.expect("app build failed");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
