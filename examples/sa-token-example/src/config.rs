//! Sa-Token configuration module
//!
//! This module defines path-based authentication rules for Sa-Token
//! and provides a custom SeaORM-based storage implementation.

use summer::app::AppBuilder;
use summer_sa_token::{lazy_storage, PathAuthBuilder, SaStorage, SaTokenConfigurator};
use std::sync::Arc;
use crate::sea_orm_storage::SeaOrmStorage;

/// Application Sa-Token configuration
///
/// Implements `SaTokenConfigurator` to define which paths require authentication
/// and to provide a custom storage backend.
pub struct SaTokenConfig;

impl SaTokenConfigurator for SaTokenConfig {
    fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
        auth
            // All paths under /user/**, /admin/**, /api/** require authentication
            .include("/user/**")
            .include("/admin/**")
            .include("/api/**")
            // These paths are public (no auth required)
            .exclude("/login")
            .exclude("/api/health") // Health check is public
            .exclude("/api/config") // Config check is public (for debugging)
            .exclude("/api/init") // Create demo table (for in-memory sqlite)
    }

    fn configure_storage(&self, _app: &AppBuilder) -> Option<Arc<dyn SaStorage>> {
        // Use lazy_storage to wrap SeaOrmStorage.
        // SeaOrmStorage is a Service with #[derive(Service)], so DbConn is auto-injected.
        // The lazy_storage wrapper defers component resolution until first use.
        Some(lazy_storage::<SeaOrmStorage>())
    }
}
