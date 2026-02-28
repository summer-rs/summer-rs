pub use inventory::submit;

use crate::{consumer::Consumers, extractor::FromMsg};
use sea_streamer::SeaMessage;
use summer::app::App;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

pub trait Handler<T>: Clone + Send + Sized + 'static {
    /// The type of future calling this handler returns.
    type Future: Future<Output = ()> + Send + 'static;

    /// Call the handler with the given request.
    fn call(self, msg: SeaMessage, app: Arc<App>) -> Self::Future;
}

/// no args handler impl
impl<F, Fut> Handler<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, _msg: SeaMessage, _app: Arc<App>) -> Self::Future {
        Box::pin(self())
    }
}

/// 1~15 args handler impl
#[rustfmt::skip]
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

macro_rules! impl_handler {
    (
        [$($ty:ident),*]
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, Fut, $($ty,)*> Handler<($($ty,)*)> for F
        where
            F: FnOnce($($ty,)*) -> Fut + Clone + Send + 'static,
            Fut: Future<Output = ()> + Send + 'static,
            $( $ty: FromMsg + Send, )*
        {
            type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

            fn call(self, msg: SeaMessage, app: Arc<App>) -> Self::Future {
                $(
                    let $ty = $ty::from_msg(&msg, &app);
                )*
                Box::pin(self($($ty,)*))
            }
        }
    };
}

all_the_tuples!(impl_handler);

pub(crate) struct BoxedHandler(Mutex<Box<dyn ErasedHandler>>);

impl Clone for BoxedHandler {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().unwrap().clone_box()))
    }
}

impl BoxedHandler {
    pub(crate) fn from_handler<H, T>(handler: H) -> Self
    where
        H: Handler<T> + Sync,
        T: 'static,
    {
        Self(Mutex::new(Box::new(MakeErasedHandler {
            handler,
            caller: |handler, msg, app| Box::pin(H::call(handler, msg, app)),
        })))
    }

    pub(crate) fn call(
        self,
        msg: SeaMessage,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        self.0.into_inner().unwrap().call(msg, app)
    }
}

pub(crate) trait ErasedHandler: Send {
    fn clone_box(&self) -> Box<dyn ErasedHandler>;

    fn call(
        self: Box<Self>,
        msg: SeaMessage,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

type HandlerCaller<H> = fn(H, SeaMessage, Arc<App>) -> Pin<Box<dyn Future<Output = ()> + Send>>;

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
        msg: SeaMessage,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        (self.caller)(self.handler, msg, app)
    }
}

/// TypeHandler is used to configure the summer-macro marked stream_listener handler
///
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
        ::summer_stream::handler::submit! {
            &$ty as &dyn ::summer_stream::handler::TypedHandlerRegistrar
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
