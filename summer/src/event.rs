//! Application event support.

use crate::{
    app::{App, AppBuilder},
    config::env::Env,
    error::Result,
    plugin::ComponentRegistry,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::{
    any::{Any, TypeId},
    future::Future,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};

/// Build-phase event listener with access to [`AppBuilder`].
#[async_trait]
pub trait BuilderEventListener: Send + Sync {
    /// Handles a type-erased event during application construction.
    async fn on_event(
        &self,
        event: Arc<dyn Any + Send + Sync>,
        app: &mut AppBuilder,
    ) -> Result<()>;
}

struct TypedBuilderListener<E, F> {
    listener: F,
    _marker: std::marker::PhantomData<fn(E)>,
}

#[async_trait]
impl<E, F, Fut> BuilderEventListener for TypedBuilderListener<E, F>
where
    E: Event,
    F: Fn(E, &mut AppBuilder) -> Fut + Send + Sync,
    Fut: Future<Output = Result<()>> + Send,
{
    async fn on_event(
        &self,
        event: Arc<dyn Any + Send + Sync>,
        app: &mut AppBuilder,
    ) -> Result<()> {
        let event = event
            .downcast::<E>()
            .expect("event listener received unexpected event type");
        (self.listener)((*event).clone(), app).await
    }
}

type BuilderListener = Arc<dyn BuilderEventListener>;

/// Runtime event listener with access to [`App`].
#[async_trait]
pub trait AppEventListener: Send + Sync {
    /// Handles a type-erased event after the application is built.
    async fn on_event(&self, event: Arc<dyn Any + Send + Sync>, app: &App) -> Result<()>;
}

struct TypedAppListener<E, F> {
    listener: F,
    _marker: std::marker::PhantomData<fn(E)>,
}

#[async_trait]
impl<E, F, Fut> AppEventListener for TypedAppListener<E, F>
where
    E: Event,
    F: Fn(E, &App) -> Fut + Send + Sync,
    Fut: Future<Output = Result<()>> + Send,
{
    async fn on_event(&self, event: Arc<dyn Any + Send + Sync>, app: &App) -> Result<()> {
        let event = event
            .downcast::<E>()
            .expect("event listener received unexpected event type");
        (self.listener)((*event).clone(), app).await
    }
}

type AppListener = Arc<dyn AppEventListener>;

/// Marker trait for events that can be published through [`EventBus`].
pub trait Event: Clone + Send + Sync + 'static {}

/// Strongly typed asynchronous event bus.
#[derive(Clone, Default)]
pub struct EventBus {
    builder_listeners: Arc<DashMap<TypeId, Vec<BuilderListener>>>,
    app_listeners: Arc<DashMap<TypeId, Vec<AppListener>>>,
}

impl EventBus {
    /// Registers a type-erased build-phase listener.
    pub fn listen_dyn<E>(&self, listener: BuilderListener)
    where
        E: Event,
    {
        self.builder_listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push(listener);
    }

    /// Registers a type-erased runtime listener.
    pub fn listen_app_dyn<E>(&self, listener: AppListener)
    where
        E: Event,
    {
        self.app_listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push(listener);
    }

    /// Registers a build-phase listener that receives [`AppBuilder`].
    pub fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &mut AppBuilder) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        self.builder_listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Arc::new(TypedBuilderListener {
                listener,
                _marker: std::marker::PhantomData,
            }));
    }

    /// Registers a runtime listener that receives [`App`].
    pub fn listen_app<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &App) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        self.app_listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Arc::new(TypedAppListener {
                listener,
                _marker: std::marker::PhantomData,
            }));
    }

    /// Publishes an event to build-phase listeners.
    pub async fn publish_builder<E>(&self, event: E, app: &mut AppBuilder) -> Result<()>
    where
        E: Event,
    {
        let listeners = self
            .builder_listeners
            .get(&TypeId::of::<E>())
            .map(|entry| entry.clone())
            .unwrap_or_default();

        let event = Arc::new(event) as Arc<dyn Any + Send + Sync>;
        for listener in listeners {
            listener.on_event(event.clone(), app).await?;
        }

        Ok(())
    }

    /// Publishes an event to runtime listeners.
    pub async fn publish_app<E>(&self, event: E, app: &App) -> Result<()>
    where
        E: Event,
    {
        let listeners = self
            .app_listeners
            .get(&TypeId::of::<E>())
            .map(|entry| entry.clone())
            .unwrap_or_default();

        let event = Arc::new(event) as Arc<dyn Any + Send + Sync>;
        for listener in listeners {
            listener.on_event(event.clone(), app).await?;
        }

        Ok(())
    }
}

/// Publishes events during application build (`AppBuilder` phase).
#[async_trait]
pub trait BuilderEventPublisher {
    /// Publishes an event to build-phase listeners.
    async fn publish<E>(&mut self, event: E) -> Result<()>
    where
        E: Event;
}

/// Publishes events on a running [`App`].
#[async_trait]
pub trait EventPublisher: Sync {
    /// Publishes an event to runtime listeners.
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event;
}

/// Subscribes to build-phase events on [`AppBuilder`].
pub trait EventSubscriber {
    /// Registers a type-erased build-phase listener.
    fn listen_dyn<E>(&self, listener: Arc<dyn BuilderEventListener>)
    where
        E: Event;

    /// Registers a listener invoked with the event and [`AppBuilder`].
    fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &mut AppBuilder) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send;
}

/// Subscribes to runtime events (after [`App`] is built).
pub trait AppEventSubscriber {
    /// Registers a type-erased runtime listener.
    fn listen_app_dyn<E>(&self, listener: Arc<dyn AppEventListener>)
    where
        E: Event;

    /// Registers a listener invoked with the event and [`App`].
    fn listen_app<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &App) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send;
}

#[async_trait]
impl BuilderEventPublisher for AppBuilder {
    async fn publish<E>(&mut self, event: E) -> Result<()>
    where
        E: Event,
    {
        let bus = self.get_expect_component::<EventBus>().clone();
        bus.publish_builder(event, self).await
    }
}

#[async_trait]
impl EventPublisher for App {
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event,
    {
        self.get_expect_component::<EventBus>()
            .publish_app(event, self)
            .await
    }
}

impl EventSubscriber for AppBuilder {
    fn listen_dyn<E>(&self, listener: Arc<dyn BuilderEventListener>)
    where
        E: Event,
    {
        self.get_expect_component::<EventBus>()
            .listen_dyn::<E>(listener);
    }

    fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &mut AppBuilder) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        self.get_expect_component::<EventBus>().listen(listener);
    }
}

impl AppEventSubscriber for AppBuilder {
    fn listen_app_dyn<E>(&self, listener: Arc<dyn AppEventListener>)
    where
        E: Event,
    {
        self.get_expect_component::<EventBus>()
            .listen_app_dyn::<E>(listener);
    }

    fn listen_app<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &App) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        self.get_expect_component::<EventBus>().listen_app(listener);
    }
}

impl AppEventSubscriber for App {
    fn listen_app_dyn<E>(&self, listener: Arc<dyn AppEventListener>)
    where
        E: Event,
    {
        self.get_expect_component::<EventBus>()
            .listen_app_dyn::<E>(listener);
    }

    fn listen_app<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E, &App) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        self.get_expect_component::<EventBus>().listen_app(listener);
    }
}

/// Describes the source used to initialize application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// Configuration loaded from a TOML file.
    File(PathBuf),
    /// Configuration loaded from an inline TOML string.
    Inline,
}

/// Published when local configuration is loaded; listeners may merge remote config into [`AppBuilder`].
#[derive(Debug, Clone)]
pub struct ConfigEvent {
    /// Currently active application environment.
    pub env: Env,
    /// Source used to load the current configuration.
    pub source: ConfigSource,
}

impl Event for ConfigEvent {}

/// Published after all plugins have been built.
#[derive(Debug, Clone)]
pub struct PluginsBuiltEvent;

impl Event for PluginsBuiltEvent {}

/// Published after service dependency injection has completed.
#[derive(Debug, Clone)]
pub struct ServicesInjectedEvent;

impl Event for ServicesInjectedEvent {}

/// Published after the application has been built and installed globally.
#[derive(Clone)]
pub struct AppBuiltEvent {
    /// Built application instance.
    pub app: Arc<App>,
}

impl Event for AppBuiltEvent {}

/// Shutdown lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPhase {
    /// Shutdown hooks are about to run.
    BeforeHooks,
    /// Shutdown hooks have completed.
    AfterHooks,
}

/// Published while the application is shutting down.
#[derive(Debug, Clone)]
pub struct ShutdownEvent {
    /// Current shutdown phase.
    pub phase: ShutdownPhase,
}

impl Event for ShutdownEvent {}

/// Protocol of a server that has started listening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerProtocol {
    /// HTTP server ([`summer-web`](https://docs.rs/summer-web)).
    Http,
    /// gRPC server ([`summer-grpc`](https://docs.rs/summer-grpc)).
    Grpc,
}

impl ServerProtocol {
    /// Canonical metadata value for Nacos instance metadata (`protocol` key).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Grpc => "grpc",
        }
    }
}

/// Published when a server (HTTP, gRPC, …) is ready to accept requests.
///
/// Plugins such as [`summer-web`](https://docs.rs/summer-web) and
/// [`summer-grpc`](https://docs.rs/summer-grpc) each publish once after bind.
/// Listeners (e.g. service discovery) may run once per protocol in the same process.
#[derive(Debug, Clone)]
pub struct ServerStartedEvent {
    /// Bound socket address (from the plugin config, often before `serve` blocks).
    pub addr: SocketAddr,
    /// Which protocol stack started; used for metadata and multi-port registration.
    pub protocol: ServerProtocol,
}

impl Event for ServerStartedEvent {}

#[cfg(test)]
mod tests {
    use super::{BuilderEventPublisher, Event, EventSubscriber};
    use crate::app::AppBuilder;
    use crate::error::Result;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct TestEvent(usize);

    impl Event for TestEvent {}

    #[derive(Clone)]
    struct OtherEvent;

    impl Event for OtherEvent {}

    #[tokio::test]
    async fn publish_dispatches_to_matching_listeners() -> Result<()> {
        let mut app = AppBuilder::default();
        let total = Arc::new(AtomicUsize::new(0));
        let total_ref = total.clone();

        app.listen(move |event: TestEvent, _app: &mut AppBuilder| {
            let total = total_ref.clone();
            async move {
                total.fetch_add(event.0, Ordering::SeqCst);
                Ok(())
            }
        });

        app.publish(TestEvent(3)).await?;
        assert_eq!(total.load(Ordering::SeqCst), 3);
        Ok(())
    }

    #[tokio::test]
    async fn publish_keeps_event_types_isolated() -> Result<()> {
        let mut app = AppBuilder::default();
        let total = Arc::new(AtomicUsize::new(0));
        let total_ref = total.clone();

        app.listen(move |_: TestEvent, _app: &mut AppBuilder| {
            let total = total_ref.clone();
            async move {
                total.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        app.publish(OtherEvent).await?;
        assert_eq!(total.load(Ordering::SeqCst), 0);
        Ok(())
    }

    #[tokio::test]
    async fn publish_dispatches_listeners_in_registration_order() -> Result<()> {
        let mut app = AppBuilder::default();
        let calls = Arc::new(Mutex::new(Vec::new()));

        let first = calls.clone();
        app.listen(move |_: TestEvent, _app: &mut AppBuilder| {
            let calls = first.clone();
            async move {
                calls.lock().await.push(1);
                Ok(())
            }
        });

        let second = calls.clone();
        app.listen(move |_: TestEvent, _app: &mut AppBuilder| {
            let calls = second.clone();
            async move {
                calls.lock().await.push(2);
                Ok(())
            }
        });

        app.publish(TestEvent(0)).await?;
        assert_eq!(*calls.lock().await, vec![1, 2]);
        Ok(())
    }
}
