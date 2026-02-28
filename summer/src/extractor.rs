use std::ops::{Deref, DerefMut};

use crate::config::Configurable;

/// Wrapper type for injecting components in #[component] macro
///
/// This is used in component function parameters to inject dependencies.
///
/// # Example
/// ```ignore
/// #[component]
/// fn create_service(
///     Component(db): Component<DbConnection>,
/// ) -> MyService {
///     MyService::new(db)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Component<T>(pub T);

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

/// Wrapper type for injecting configuration in #[component] macro
///
/// This is used in component function parameters to inject configuration.
///
/// # Example
/// ```ignore
/// #[component]
/// fn create_db(
///     Config(config): Config<DbConfig>,
/// ) -> DbConnection {
///     DbConnection::new(&config)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Config<T>(pub T)
where
    T: serde::de::DeserializeOwned + Configurable;

impl<T> Deref for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
