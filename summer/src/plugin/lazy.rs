use std::sync::{Arc, RwLock};
use crate::error::Result;
use crate::plugin::ComponentRegistry;
use crate::App;

/// A lazy-loaded component wrapper that allows circular dependencies.
/// The component is resolved on first access from the global app registry.
///
/// # Usage
/// When you have circular dependencies between services, declare the field type
/// as `LazyComponent<T>`. No `#[inject]` attribute is needed - it's automatically detected:
///
/// ```ignore
/// #[derive(Clone, Service)]
/// struct UserService {
///     #[inject(config)]
///     config: UserConfig,
///     better_user: LazyComponent<BetterUserService>,  // Automatically detected as lazy
/// }
///
/// #[derive(Clone, Service)]
/// struct BetterUserService {
///     #[inject(component)]
///     user_service: UserService,  // This creates a circular dependency
/// }
///
/// // Access the lazy component:
/// impl UserService {
///     fn use_better_service(&self) -> Result<String> {
///         let better = self.better_user.get()?;  // Resolves on first access
///         Ok(better.do_something())
///     }
/// }
/// ```
///
/// # Why LazyComponent<T>?
/// The field type must be `LazyComponent<T>` (not just `T`) because:
/// - It breaks the circular dependency at the type level
/// - It allows the component to be resolved after all services are registered
/// - The type itself signals lazy initialization - no attribute needed
#[derive(Clone)]
pub struct LazyComponent<T: Clone + Send + Sync + 'static> {
    component: Arc<RwLock<Option<T>>>,
}

impl<T: Clone + Send + Sync + 'static> Default for LazyComponent<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + Sync + 'static> LazyComponent<T> {
    /// Creates a new lazy component wrapper
    pub fn new() -> Self {
        Self {
            component: Arc::new(RwLock::new(None)),
        }
    }

    /// Gets the component, initializing it from the global registry if needed
    pub fn get(&self) -> Result<T> {
        {
            let guard = self.component.read().unwrap();
            if let Some(component) = guard.as_ref() {
                return Ok(component.clone());
            }
        }
        
        let mut guard = self.component.write().unwrap();
        
        if let Some(component) = guard.as_ref() {
            return Ok(component.clone());
        }
        
        let component = App::global().try_get_component::<T>()?;
        *guard = Some(component.clone());
        Ok(component)
    }

    /// Attempts to get the component if it's already been initialized
    pub fn get_if_initialized(&self) -> Option<T> {
        self.component.read().unwrap().clone()
    }
}

impl<T: Clone + Send + Sync + 'static> std::ops::Deref for LazyComponent<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        panic!("LazyComponent cannot be dereferenced directly. Use .get() method instead.")
    }
}

impl<T: Clone + Send + Sync + 'static> std::fmt::Debug for LazyComponent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyComponent")
            .field("initialized", &self.component.read().unwrap().is_some())
            .finish()
    }
}
