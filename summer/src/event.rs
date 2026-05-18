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
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};

type EventFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
type ErasedListener = Arc<dyn Fn(Arc<dyn Any + Send + Sync>) -> EventFuture + Send + Sync>;

/// Marker trait for events that can be published through [`EventBus`].
pub trait Event: Clone + Send + Sync + 'static {}

/// Strongly typed asynchronous event bus.
#[derive(Clone, Default)]
pub struct EventBus {
    listeners: Arc<DashMap<TypeId, Vec<ErasedListener>>>,
}

impl EventBus {
    /// Registers a listener for the specified event type.
    pub fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let erased = Arc::new(move |event: Arc<dyn Any + Send + Sync>| {
            let event = event
                .downcast::<E>()
                .expect("event listener received unexpected event type");
            Box::pin(listener((*event).clone())) as EventFuture
        });

        self.listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push(erased);
    }

    /// Publishes an event to all listeners registered for its type.
    pub async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event,
    {
        let listeners = self
            .listeners
            .get(&TypeId::of::<E>())
            .map(|entry| entry.clone())
            .unwrap_or_default();

        let event = Arc::new(event) as Arc<dyn Any + Send + Sync>;
        for listener in listeners {
            listener(event.clone()).await?;
        }

        Ok(())
    }
}

/// Extension trait for publishing typed events.
#[async_trait]
pub trait EventPublisher: Sync {
    /// Publishes an event to all matching listeners.
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event;
}

/// Extension trait for subscribing to typed events.
pub trait EventSubscriber {
    /// Registers a listener for the specified event type.
    fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static;
}

#[async_trait]
impl EventPublisher for EventBus {
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event,
    {
        EventBus::publish(self, event).await
    }
}

impl EventSubscriber for EventBus {
    fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        EventBus::listen(self, listener)
    }
}

#[async_trait]
impl EventPublisher for App {
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event,
    {
        self.get_expect_component::<EventBus>().publish(event).await
    }
}

impl EventSubscriber for App {
    fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.get_expect_component::<EventBus>().listen(listener)
    }
}

#[async_trait]
impl EventPublisher for AppBuilder {
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event,
    {
        self.get_expect_component::<EventBus>().publish(event).await
    }
}

impl EventSubscriber for AppBuilder {
    fn listen<E, F, Fut>(&self, listener: F)
    where
        E: Event,
        F: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.get_expect_component::<EventBus>().listen(listener)
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

/// Published when application configuration is available to plugins.
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
    Starting,
    /// Shutdown hooks have completed.
    Completed,
}

/// Published while the application is shutting down.
#[derive(Debug, Clone)]
pub struct ShutdownEvent {
    /// Current shutdown phase.
    pub phase: ShutdownPhase,
}

impl Event for ShutdownEvent {}

#[cfg(test)]
mod tests {
    use super::{Event, EventBus};
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
        let bus = EventBus::default();
        let total = Arc::new(AtomicUsize::new(0));
        let total_ref = total.clone();

        bus.listen(move |event: TestEvent| {
            let total = total_ref.clone();
            async move {
                total.fetch_add(event.0, Ordering::SeqCst);
                Ok(())
            }
        });

        bus.publish(TestEvent(3)).await?;
        assert_eq!(total.load(Ordering::SeqCst), 3);
        Ok(())
    }

    #[tokio::test]
    async fn publish_keeps_event_types_isolated() -> Result<()> {
        let bus = EventBus::default();
        let total = Arc::new(AtomicUsize::new(0));
        let total_ref = total.clone();

        bus.listen(move |_: TestEvent| {
            let total = total_ref.clone();
            async move {
                total.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        bus.publish(OtherEvent).await?;
        assert_eq!(total.load(Ordering::SeqCst), 0);
        Ok(())
    }

    #[tokio::test]
    async fn publish_dispatches_listeners_in_registration_order() -> Result<()> {
        let bus = EventBus::default();
        let calls = Arc::new(Mutex::new(Vec::new()));

        let first = calls.clone();
        bus.listen(move |_: TestEvent| {
            let calls = first.clone();
            async move {
                calls.lock().await.push(1);
                Ok(())
            }
        });

        let second = calls.clone();
        bus.listen(move |_: TestEvent| {
            let calls = second.clone();
            async move {
                calls.lock().await.push(2);
                Ok(())
            }
        });

        bus.publish(TestEvent(0)).await?;
        assert_eq!(*calls.lock().await, vec![1, 2]);
        Ok(())
    }
}
