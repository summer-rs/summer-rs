pub use axum::extract::*;

use crate::error::{Result, WebError};
use crate::AppState;
use anyhow::Context;
use axum::http::request::Parts;
use summer::config::{ConfigRegistry, Configurable};
use summer::plugin::ComponentRegistry;
use summer::App;
use std::ops::{Deref, DerefMut};
use std::result::Result as StdResult;
use std::sync::Arc;

/// Extending the functionality of RequestParts
pub trait RequestPartsExt {
    /// get AppState
    fn get_app_state(&self) -> &AppState;

    /// get Component
    fn get_component<T: Clone + Send + Sync + 'static>(&self) -> Result<T>;

    /// get Config
    fn get_config<T: serde::de::DeserializeOwned + Configurable>(&self) -> Result<T>;
}

impl RequestPartsExt for Parts {
    fn get_app_state(&self) -> &AppState {
        self.extensions
            .get::<AppState>()
            .expect("extract app state from extension failed")
    }

    fn get_component<T: Clone + Send + Sync + 'static>(&self) -> Result<T> {
        Ok(self
            .get_app_state()
            .app
            .try_get_component()
            .context("get_component failed")?)
    }

    fn get_config<T: serde::de::DeserializeOwned + Configurable>(&self) -> Result<T> {
        self.get_app_state()
            .app
            .get_config::<T>()
            .map_err(|e| WebError::ConfigDeserializeErr(std::any::type_name::<T>(), Box::new(e)))
    }
}

/// Extract the components registered by the plugin from AppState
pub struct Component<T: Clone>(pub T);

impl<T, S> FromRequestParts<S> for Component<T>
where
    T: Clone + Send + Sync + 'static,
    S: Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> StdResult<Self, Self::Rejection> {
        parts.get_component::<T>().map(|c| Component(c))
    }
}

#[cfg(feature = "openapi")]
impl<T: Clone> aide::OperationInput for Component<T> {}

impl<T: Clone> Deref for Component<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> DerefMut for Component<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Config<T>(pub T)
where
    T: serde::de::DeserializeOwned + Configurable;

impl<T, S> FromRequestParts<S> for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
    S: Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> StdResult<Self, Self::Rejection> {
        parts.get_config().map(|c| Config(c))
    }
}

#[cfg(feature = "openapi")]
impl<T> aide::OperationInput for Config<T> where T: serde::de::DeserializeOwned + Configurable {}

impl<T> Deref for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Extract Arc<App>
pub struct AppRef(pub Arc<App>);

impl<S> FromRequestParts<S> for AppRef
where
    S: Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> StdResult<Self, Self::Rejection> {
        Ok(Self(parts.get_app_state().app.clone()))
    }
}

#[cfg(feature = "openapi")]
impl aide::OperationInput for AppRef {}

#[cfg(feature = "socket_io")]
mod socketio_extractors {
    use super::*;
    use crate::socketioxide::adapter::LocalAdapter;
    use crate::socketioxide::extract::HttpExtension;
    use crate::socketioxide::handler::connect::FromConnectParts;
    use crate::socketioxide::handler::disconnect::FromDisconnectParts;
    use crate::socketioxide::handler::message::FromMessageParts;
    use crate::socketioxide::socket::{DisconnectReason, Socket};
    use socketioxide::handler::Value;
    use std::sync::Arc;

    #[derive(Debug)]
    pub struct ComponentExtractError(pub String);

    impl std::fmt::Display for ComponentExtractError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Component extraction error: {}", self.0)
        }
    }

    impl std::error::Error for ComponentExtractError {}

    impl<T> FromConnectParts<LocalAdapter> for Component<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        type Error = ComponentExtractError;

        fn from_connect_parts(
            s: &Arc<Socket<LocalAdapter>>,
            _auth: &Option<Value>,
        ) -> StdResult<Self, Self::Error> {
            let app = HttpExtension::<AppState>::from_connect_parts(s, _auth)
                .map_err(|e| ComponentExtractError(format!("Failed to extract AppState: {e}")))?;

            app.app
                .try_get_component()
                .map(|c| Component(c))
                .map_err(|e| ComponentExtractError(format!("Failed to get component: {e}")))
        }
    }

    impl<T> FromMessageParts<LocalAdapter> for Component<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        type Error = ComponentExtractError;

        fn from_message_parts(
            s: &Arc<Socket<LocalAdapter>>,
            _data: &mut Value,
            _ack_id: &Option<i64>,
        ) -> StdResult<Self, Self::Error> {
            let app = HttpExtension::<AppState>::from_message_parts(s, _data, _ack_id)
                .map_err(|e| ComponentExtractError(format!("Failed to extract AppState: {e}")))?;

            app.app
                .try_get_component()
                .map(|c| Component(c))
                .map_err(|e| ComponentExtractError(format!("Failed to get component: {e}")))
        }
    }

    impl<T> FromDisconnectParts<LocalAdapter> for Component<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        type Error = ComponentExtractError;

        fn from_disconnect_parts(
            s: &Arc<Socket<LocalAdapter>>,
            reason: DisconnectReason,
        ) -> StdResult<Self, Self::Error> {
            let app = HttpExtension::<AppState>::from_disconnect_parts(s, reason)
                .map_err(|e| ComponentExtractError(format!("Failed to extract AppState: {e}")))?;

            app.app
                .try_get_component()
                .map(|c| Component(c))
                .map_err(|e| ComponentExtractError(format!("Failed to get component: {e}")))
        }
    }
}
