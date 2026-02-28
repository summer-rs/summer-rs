pub use inventory::submit;

use crate::JobScheduler;
use crate::{extractor::FromApp, JobId, Jobs};
use summer::app::App;
use std::pin::Pin;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};

pub trait Handler<T>: Clone + Send + Sized + 'static {
    /// The type of future calling this handler returns.
    type Future: Future<Output = ()> + Send + 'static;

    /// Call the handler with the given request.
    fn call(self, job_id: JobId, scheduler: JobScheduler, app: Arc<App>) -> Self::Future;
}

/// no args handler impl
impl<F, Fut> Handler<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn call(self, _job_id: JobId, _scheduler: JobScheduler, _app: Arc<App>) -> Self::Future {
        Box::pin(async move {
            self().await;
        })
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
            Fut: Future<Output = ()> + Send,
            $( $ty: FromApp + Send, )*
        {
            type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

            fn call(self, job_id: JobId, scheduler: JobScheduler, app: Arc<App>) -> Self::Future {
                Box::pin(async move {
                    $(
                        let $ty = $ty::from_app(&job_id, &scheduler, &app).await;
                    )*

                    self($($ty,)*).await;
                })
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
            caller: |handler, job_id, jobs, app| Box::pin(H::call(handler, job_id, jobs, app)),
        })))
    }

    pub(crate) fn call(
        self,
        job_id: JobId,
        scheduler: JobScheduler,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        self.0.into_inner().unwrap().call(job_id, scheduler, app)
    }
}

pub(crate) trait ErasedHandler: Send {
    fn clone_box(&self) -> Box<dyn ErasedHandler>;

    fn call(
        self: Box<Self>,
        job_id: JobId,
        scheduler: JobScheduler,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

type HandlerCaller<H> =
    fn(H, JobId, JobScheduler, Arc<App>) -> Pin<Box<dyn Future<Output = ()> + Send>>;

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
        job_id: JobId,
        scheduler: JobScheduler,
        app: Arc<App>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        (self.caller)(self.handler, job_id, scheduler, app)
    }
}

/// TypeHandler is used to configure the summer-macro marked job handler
pub trait TypedHandlerRegistrar: Send + Sync + 'static {
    fn install_job(&self, jobs: Jobs) -> Jobs;
}

pub trait TypedJob {
    fn typed_job<F: TypedHandlerRegistrar>(self, factory: F) -> Self;
}

impl TypedJob for Jobs {
    fn typed_job<F: TypedHandlerRegistrar>(self, factory: F) -> Self {
        factory.install_job(self)
    }
}

inventory::collect!(&'static dyn TypedHandlerRegistrar);

#[macro_export]
macro_rules! submit_typed_handler {
    ($ty:ident) => {
        ::summer_job::handler::submit! {
            &$ty as &dyn ::summer_job::handler::TypedHandlerRegistrar
        }
    };
}

pub fn auto_jobs() -> Jobs {
    let mut jobs = Jobs::new();
    for factory in inventory::iter::<&dyn TypedHandlerRegistrar> {
        jobs = factory.install_job(jobs);
    }
    jobs
}
