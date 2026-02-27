//! Service is a special Component that supports dependency injection at compile time
#![doc = include_str!("../../DI.md")]
use crate::app::AppBuilder;
use crate::config::ConfigRegistry;
use crate::error::Result;
use crate::plugin::ComponentRegistry;

pub use spring_macros::Service;

/// Service is a special Component that can inject dependent Components as field members
/// ```rust
/// use spring::plugin::service::Service;
/// use spring_sqlx::ConnectPool;
///
/// #[derive(Clone, Service)]
/// struct UserService {
///     #[inject(component)]
///     db: ConnectPool
/// }
/// ```
pub trait Service: Clone + Sized + 'static {
    /// Construct the Service component
    fn build<R>(registry: &R) -> Result<Self>
    where
        R: ComponentRegistry + ConfigRegistry;
}

//////////////////////////////////////////////////
/// Install the Service component into the App
pub trait ServiceRegistrar: Send + Sync + 'static {
    /// Install the Service component into the App
    fn install_service(&self, app: &mut AppBuilder) -> Result<()>;
}

inventory::collect!(&'static dyn ServiceRegistrar);

/// auto_config
#[macro_export]
macro_rules! submit_service {
    ($ty:ident) => {
        ::spring::submit_inventory! {
            &$ty as &dyn ::spring::plugin::service::ServiceRegistrar
        }
    };
}

/// Find all ServiceRegistrar and install them into the app
pub fn auto_inject_service(app: &mut AppBuilder) -> Result<()> {
    let registrars: Vec<&'static &dyn ServiceRegistrar> =
        inventory::iter::<&dyn ServiceRegistrar>().collect();
    let total = registrars.len();
    let mut pending: Vec<&'static &dyn ServiceRegistrar> = registrars;
    let mut installed = 0;

    while !pending.is_empty() {
        let mut next_pending = Vec::new();
        let mut progress_made = false;

        for registrar in pending {
            match registrar.install_service(app) {
                Ok(()) => {
                    installed += 1;
                    progress_made = true;
                }
                Err(_) => {
                    next_pending.push(registrar);
                }
            }
        }

        if !progress_made && !next_pending.is_empty() {
            if let Some(first) = next_pending.first() {
                return first.install_service(app);
            }
        }

        pending = next_pending;
    }

    log::debug!("Installed {installed}/{total} services");
    Ok(())
}
