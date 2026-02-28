// Centralized re-exports for the crate root.
//
// This module is intentionally NOT public. We `pub use prelude::*;` from `lib.rs`
// so users only see the short `summer_sa_token::X` paths (no extra `::prelude::`).

// Re-export upstream crates (full access when needed).
pub use sa_token_adapter;
pub use sa_token_core;

// Common upstream types
pub use sa_token_adapter::{FrameworkAdapter, SaStorage};
pub use sa_token_core::{SaTokenManager, StpUtil};

// summer-sa-token config types
pub use crate::config::{CoreConfig, SaTokenConfig, TokenStyle};

// summer-sa-token custom storage
pub use crate::custom_storage::lazy_storage;

// Axum integration (plugin specific)
pub use sa_token_plugin_axum::adapter::{AxumRequestAdapter, AxumResponseAdapter};
pub use sa_token_plugin_axum::layer::extract_token_from_request;
pub use sa_token_plugin_axum::{LoginIdExtractor, OptionalSaTokenExtractor, SaTokenExtractor};
pub use sa_token_plugin_axum::{
    SaCheckLoginLayer, SaCheckLoginMiddleware, SaCheckPermissionLayer, SaCheckPermissionMiddleware,
    SaTokenLayer, SaTokenMiddleware, SaTokenState, SaTokenStateBuilder,
};

// Procedural macros (WebError compatible)
pub use summer_macros::{
    sa_check_login, sa_check_permission, sa_check_permissions_and, sa_check_permissions_or,
    sa_check_role, sa_check_roles_and, sa_check_roles_or, sa_ignore,
};

// Storage implementations
#[cfg(feature = "memory")]
pub use sa_token_storage_memory::MemoryStorage;

// Configurator types
pub use crate::configurator::{PathAuthBuilder, SaTokenAuthConfigurator, SaTokenConfigurator};
