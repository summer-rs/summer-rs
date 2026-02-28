use std::sync::Arc;
use std::time::Duration;

use summer::async_trait;
use sa_token_adapter::storage::{SaStorage, StorageResult};
use summer::plugin::LazyComponent;

/// Custom Sa-Token storage component wrapper.
///
/// We store a concrete component type (instead of `Arc<dyn SaStorage>`) to avoid IDE/import
/// confusion and to match the common "component is a named type" style used in this repo.
#[derive(Clone)]
pub struct SaTokenStorage(pub Arc<dyn SaStorage>);

impl SaTokenStorage {
    pub fn new(storage: Arc<dyn SaStorage>) -> Self {
        Self(storage)
    }
}

impl std::ops::Deref for SaTokenStorage {
    type Target = dyn SaStorage;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl From<SaTokenStorage> for Arc<dyn SaStorage> {
    fn from(value: SaTokenStorage) -> Self {
        value.0
    }
}

// ============================================================================
// LazyStorage - Lazy-loaded storage wrapper for user-defined Service components
// ============================================================================

/// Create a lazy-loaded storage that resolves a Service component at runtime.
///
/// This function is used when you define your storage as a `#[derive(Service)]` component.
/// The storage will be lazily resolved from the global component registry when first accessed.
///
/// # Example
///
/// ```rust,ignore
/// use summer::plugin::service::Service;
/// use summer_sa_token::{lazy_storage, SaStorage, SaTokenConfigurator};
///
/// // Define your storage as a Service - DbConn is auto-injected
/// #[derive(Clone, Service)]
/// pub struct MyStorage {
///     #[inject(component)]
///     conn: DbConn,
/// }
///
/// impl SaStorage for MyStorage { /* ... */ }
///
/// // In your configurator
/// impl SaTokenConfigurator for MyConfig {
///     fn configure_storage(&self, _app: &AppBuilder) -> Option<Arc<dyn SaStorage>> {
///         Some(lazy_storage::<MyStorage>())
///     }
/// }
/// ```
pub fn lazy_storage<T>() -> Arc<dyn SaStorage>
where
    T: SaStorage + Clone + Send + Sync + 'static,
{
    Arc::new(LazyStorageWrapper::<T>::new())
}

/// Internal wrapper that lazily resolves a storage component.
///
/// This struct uses `LazyComponent<T>` internally to defer component resolution
/// until the storage is actually used (after all Services are registered).
#[derive(Clone)]
struct LazyStorageWrapper<T: Clone + Send + Sync + 'static> {
    inner: LazyComponent<T>,
}

impl<T: Clone + Send + Sync + 'static> LazyStorageWrapper<T> {
    fn new() -> Self {
        Self {
            inner: LazyComponent::new(),
        }
    }
}

#[async_trait]
impl<T: SaStorage + Clone + Send + Sync + 'static> SaStorage for LazyStorageWrapper<T> {
    async fn get(&self, key: &str) -> StorageResult<Option<String>> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.get(key).await
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> StorageResult<()> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.set(key, value, ttl).await
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.delete(key).await
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.exists(key).await
    }

    async fn expire(&self, key: &str, ttl: Duration) -> StorageResult<()> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.expire(key, ttl).await
    }

    async fn ttl(&self, key: &str) -> StorageResult<Option<Duration>> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.ttl(key).await
    }

    async fn mget(&self, keys: &[&str]) -> StorageResult<Vec<Option<String>>> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.mget(keys).await
    }

    async fn mset(&self, items: &[(&str, &str)], ttl: Option<Duration>) -> StorageResult<()> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.mset(items, ttl).await
    }

    async fn mdel(&self, keys: &[&str]) -> StorageResult<()> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.mdel(keys).await
    }

    async fn incr(&self, key: &str) -> StorageResult<i64> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.incr(key).await
    }

    async fn decr(&self, key: &str) -> StorageResult<i64> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.decr(key).await
    }

    async fn clear(&self) -> StorageResult<()> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.clear().await
    }

    async fn keys(&self, pattern: &str) -> StorageResult<Vec<String>> {
        let storage = self.inner.get().map_err(|e| {
            sa_token_adapter::storage::StorageError::OperationFailed(e.to_string())
        })?;
        storage.keys(pattern).await
    }
}
