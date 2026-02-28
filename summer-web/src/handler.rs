use crate::Router;

pub use inventory::submit;

/// TypeHandler is used to configure the summer-macro marked route handler
pub trait TypedHandlerRegistrar: Send + Sync + 'static {
    /// install route
    fn install_route(&self, router: Router) -> Router;
}

/// Add typed routes marked with procedural macros
pub trait TypeRouter {
    /// Add typed routes marked with procedural macros
    fn typed_route<F: TypedHandlerRegistrar>(self, factory: F) -> Self;
}

impl TypeRouter for Router {
    fn typed_route<F: TypedHandlerRegistrar>(self, factory: F) -> Self {
        factory.install_route(self)
    }
}

inventory::collect!(&'static dyn TypedHandlerRegistrar);

/// auto_config
#[macro_export]
macro_rules! submit_typed_handler {
    ($ty:ident) => {
        ::summer_web::handler::submit! {
            &$ty as &dyn ::summer_web::handler::TypedHandlerRegistrar
        }
    };
}

#[cfg(feature = "socket_io")]
#[macro_export]
macro_rules! submit_socketio_handler {
    ($ty:ident) => {
        ::summer_web::handler::submit! {
            &$ty as &dyn ::summer_web::handler::SocketIOHandlerRegistrar
        }
    };
}

/// auto_config
pub fn auto_router() -> Router {
    #[cfg(feature = "openapi")]
    crate::enable_openapi();

    let mut router = Router::new();
    for handler in inventory::iter::<&dyn TypedHandlerRegistrar> {
        router = handler.install_route(router);
    }
    router
}

#[cfg(feature = "socket_io")]
pub trait SocketIOHandlerRegistrar: Send + Sync + 'static {
    fn install_socketio_handlers(&self, socket: &crate::socketioxide::extract::SocketRef);
}

#[cfg(feature = "socket_io")]
inventory::collect!(&'static dyn SocketIOHandlerRegistrar);

#[cfg(feature = "socket_io")]
pub fn auto_socketio_setup(socket: &crate::socketioxide::extract::SocketRef) {
    for handler in inventory::iter::<&dyn SocketIOHandlerRegistrar> {
        handler.install_socketio_handlers(socket);
    }
}
