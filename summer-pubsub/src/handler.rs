pub use inventory::submit;

use crate::consumer::Consumers;
use crate::extractor::FromPubSubMsg;
use google_cloud_pubsub::subscriber::handler::Handler;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use summer::app::App;

/// Internal type surfaced only to the `FromPubSubMsg` machinery; do not rely on it in application code.
#[doc(hidden)]
pub struct PubSubEnvelope {
    pub(crate) ack: Arc<Mutex<Option<Handler>>>,
}

impl PubSubEnvelope {
    pub(crate) fn new(handler: Handler) -> Self {
        Self {
            ack: Arc::new(Mutex::new(Some(handler))),
        }
    }
}

impl Drop for PubSubEnvelope {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.ack.lock() {
            if let Some(h) = slot.take() {
                h.ack();
            }
        }
    }
}

pub trait HandlerArgs<T>: Clone + Send + Sized + 'static {
    type Future: Future<Output = ()> + Send + 'static;

    fn call(self, grpc: google_cloud_pubsub::model::Message, env: PubSubEnvelope, app: Arc<App>) -> Self::Future;
}

impl<F, Fut> HandlerArgs<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(
        self,
        _grpc: google_cloud_pubsub::model::Message,
        _env: PubSubEnvelope,
        _app: Arc<App>,
    ) -> Self::Future {
        Box::pin(self())
    }
}

macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!([T1]);
        $name!([T1, T2]);
        $name!([T1, T2, T3]);
        $name!([T1, T2, T3, T4]);
        $name!([T1, T2, T3, T4, T5]);
        $name!([T1, T2, T3, T4, T5, T6]);
        $name!([T1, T2, T3, T4, T5, T6, T7]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15]);
    };
}

macro_rules! impl_handler_args {
    (
        [$($ty:ident),*]
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, Fut, $($ty,)*> HandlerArgs<($($ty,)*)> for F
        where
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = ()> + Send + 'static,
            $( $ty: FromPubSubMsg + Send, )*
        {
            type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

            fn call(self, grpc: google_cloud_pubsub::model::Message, env: PubSubEnvelope, app: Arc<App>) -> Self::Future {
                $(
                    let $ty = $ty::from_pubsub(&grpc, &env, &app);
                )*
                Box::pin(self($($ty,)*))
            }
        }
    };
}

all_the_tuples!(impl_handler_args);

pub(crate) struct BoxedHandler(std::sync::Mutex<Box<dyn ErasedHandler>>);

impl Clone for BoxedHandler {
    fn clone(&self) -> Self {
        Self(std::sync::Mutex::new(self.0.lock().unwrap().clone_box()))
    }
}

impl BoxedHandler {
    pub(crate) fn from_handler<H, T>(handler: H) -> Self
    where
        H: HandlerArgs<T> + Sync,
        T: 'static,
    {
        Self(std::sync::Mutex::new(Box::new(MakeErasedHandler {
            handler,
            caller: |handler, grpc, env, app| Box::pin(H::call(handler, grpc, env, app)),
        })))
    }

    pub(crate) fn call(
        self,
        grpc: google_cloud_pubsub::model::Message,
        env: PubSubEnvelope,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        self.0.into_inner().unwrap().call(grpc, env, app)
    }
}

pub(crate) trait ErasedHandler: Send {
    fn clone_box(&self) -> Box<dyn ErasedHandler>;

    fn call(
        self: Box<Self>,
        grpc: google_cloud_pubsub::model::Message,
        env: PubSubEnvelope,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

type HandlerCaller<H> = fn(
    H,
    google_cloud_pubsub::model::Message,
    PubSubEnvelope,
    Arc<App>,
) -> Pin<Box<dyn Future<Output = ()> + Send>>;

pub(crate) struct MakeErasedHandler<H> {
    pub(crate) handler: H,
    pub(crate) caller: HandlerCaller<H>,
}

impl<H> Clone for MakeErasedHandler<H>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            caller: self.caller,
        }
    }
}

impl<H> ErasedHandler for MakeErasedHandler<H>
where
    H: Clone + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn ErasedHandler> {
        Box::new(self.clone())
    }

    fn call(
        self: Box<Self>,
        grpc: google_cloud_pubsub::model::Message,
        env: PubSubEnvelope,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        (self.caller)(self.handler, grpc, env, app)
    }
}

pub trait TypedHandlerRegistrar: Send + Sync + 'static {
    fn install_consumer(&self, jobs: Consumers) -> Consumers;
}

pub trait TypedConsumer {
    fn typed_consumer<F: TypedHandlerRegistrar>(self, factory: F) -> Self;
}

impl TypedConsumer for Consumers {
    fn typed_consumer<F: TypedHandlerRegistrar>(self, factory: F) -> Self {
        factory.install_consumer(self)
    }
}

inventory::collect!(&'static dyn TypedHandlerRegistrar);

#[macro_export]
macro_rules! submit_typed_handler {
    ($ty:ident) => {
        ::summer_pubsub::handler::submit! {
            &$ty as &dyn ::summer_pubsub::handler::TypedHandlerRegistrar
        }
    };
}

pub fn auto_consumers() -> Consumers {
    let mut consumers = Consumers::new();
    for factory in inventory::iter::<&dyn TypedHandlerRegistrar> {
        consumers = factory.install_consumer(consumers);
    }
    consumers
}
