//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-sa-token)
#![doc = include_str!("../README.md")]
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]


mod custom_storage;
mod config;
mod configurator;
mod prelude;
#[cfg(feature = "with-summer-redis")]
pub mod storage;

use crate::config::SaTokenConfig as summerSaTokenConfig;
use summer::app::AppBuilder;
use summer::async_trait;
use summer::config::ConfigRegistry;
#[cfg(feature = "with-web")]
use summer::plugin::ComponentRegistry;
use summer::plugin::{MutableComponentRegistry, Plugin};
#[cfg(feature = "with-web")]
use summer_web::LayerConfigurator;

// ============================================================================
// Re-exports: Users only need to import summer-sa-token
// ============================================================================
pub use prelude::*;

#[cfg(feature = "with-web")]
use sa_token_core::PathAuthConfig as CorePathAuthConfig;

/// Sa-Token plugin for summer-rs
///
/// This plugin initializes the Sa-Token authentication system and registers
/// `SaTokenState` as a component that can be injected into handlers.
pub struct SaTokenPlugin;

#[async_trait]
impl Plugin for SaTokenPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<summerSaTokenConfig>()
            .expect("sa-token plugin config load failed");

        tracing::info!("Initializing Sa-Token plugin...");

        let state = Self::create_state(app, config)
            .await
            .expect("sa-token state creation failed");

        tracing::debug!(
            "SaTokenState manager.config.token_name = {}",
            state.manager.config.token_name
        );

        tracing::info!("Sa-Token plugin initialized successfully");

        // Register SaTokenState as a component
        app.add_component(state.clone());

        // Automatically register SaTokenLayer as a router layer
        // This middleware will extract tokens from requests and validate them
        #[cfg(feature = "with-web")]
        {
            // Get path-based authentication configuration from component
            if let Some(config) = app.get_component::<CorePathAuthConfig>() {
                tracing::info!("Registering SaTokenLayer with path-based authentication");
                app.add_router_layer(move |router| {
                    router.layer(SaTokenLayer::with_path_auth(state.clone(), config.clone()))
                });
            } else {
                tracing::info!("Registering SaTokenLayer as router middleware (no path config)");
                app.add_router_layer(move |router| router.layer(SaTokenLayer::new(state.clone())));
            }
        }
    }

    fn name(&self) -> &str {
        "summer_sa_token::SaTokenPlugin"
    }

    #[cfg(feature = "with-summer-redis")]
    fn dependencies(&self) -> Vec<&str> {
        vec!["summer_redis::RedisPlugin"]
    }
}

impl SaTokenPlugin {
    /// Create SaTokenState from configuration
    ///
    /// Uses SaTokenConfig::builder() from sa-token-core which supports most config fields
    /// and automatically initializes StpUtil.
    ///
    /// Note: The following fields are not supported by the builder (using defaults):
    /// - is_log, is_read_cookie, is_read_header, is_read_body
    async fn create_state(
        app: &AppBuilder,
        config: summerSaTokenConfig,
    ) -> anyhow::Result<SaTokenState> {
        // Configure storage based on features
        let storage = Self::configure_storage(app, &config).await?;

        tracing::debug!(
            "Sa-Token config: token_name={}, timeout={}, auto_renew={}, is_concurrent={}, is_share={}",
            config.token_name, config.timeout, config.auto_renew, config.is_concurrent, config.is_share
        );

        // Use SaTokenConfig::builder() from sa-token-core which supports more config fields
        // and automatically initializes StpUtil on build()
        let mut builder = sa_token_core::SaTokenConfig::builder()
            .storage(storage)
            .token_name(config.token_name)
            .timeout(config.timeout)
            .active_timeout(config.active_timeout)
            .auto_renew(config.auto_renew)
            .is_concurrent(config.is_concurrent)
            .is_share(config.is_share)
            .token_style(config.token_style.into())
            .enable_nonce(config.enable_nonce)
            .nonce_timeout(config.nonce_timeout)
            .enable_refresh_token(config.enable_refresh_token)
            .refresh_token_timeout(config.refresh_token_timeout);

        // Set optional fields
        if let Some(prefix) = config.token_prefix {
            builder = builder.token_prefix(prefix);
        }
        if let Some(key) = config.jwt_secret_key {
            builder = builder.jwt_secret_key(key);
        }
        if let Some(algorithm) = config.jwt_algorithm {
            builder = builder.jwt_algorithm(algorithm);
        }
        if let Some(issuer) = config.jwt_issuer {
            builder = builder.jwt_issuer(issuer);
        }
        if let Some(audience) = config.jwt_audience {
            builder = builder.jwt_audience(audience);
        }

        // build() creates SaTokenManager and auto-initializes StpUtil
        let manager = builder.build();

        // Create SaTokenState from manager
        Ok(SaTokenState::from_manager(manager))
    }

    /// Configure storage backend based on features and configuration
    ///
    /// Priority:
    /// 0. user-provided [`SaTokenStorage`] component (if present)
    /// 1. summer-redis component (if with-summer-redis feature enabled)
    /// 2. memory storage (if memory feature enabled)
    ///
    /// When both features are enabled, with-summer-redis takes priority.
    #[allow(unused_variables)]
    async fn configure_storage(
        app: &AppBuilder,
        config: &summerSaTokenConfig,
    ) -> anyhow::Result<std::sync::Arc<dyn SaStorage>> {
        // Priority 0: user-provided storage component (registered via `sa_token_configure(...)`
        // or manually with `app.add_component(SaTokenStorage::new(storage))`).
        if let Some(storage) = app.get_component::<custom_storage::SaTokenStorage>() {
            tracing::info!("Using custom SaStorage component");
            return Ok(storage.into());
        }

        // Priority 1: Use summer-redis component if available
        #[cfg(feature = "with-summer-redis")]
        {
            if let Some(redis) = app.get_component::<summer_redis::Redis>() {
                tracing::info!("Using summerRedisStorage (reusing summer-redis connection)");
                let storage = storage::summerRedisStorage::new(redis);
                Ok(std::sync::Arc::new(storage))
            } else {
                anyhow::bail!(
                    "Feature 'with-summer-redis' is enabled but RedisPlugin is not added. \
                     Please add RedisPlugin before SaTokenPlugin."
                );
            }
        }

        // Priority 2: Fall back to memory storage (only when with-summer-redis is not enabled)
        #[cfg(all(feature = "memory", not(feature = "with-summer-redis")))]
        {
            tracing::info!("Using Memory storage");
            Ok(std::sync::Arc::new(MemoryStorage::new()))
        }

        // No storage available
        #[cfg(not(any(feature = "memory", feature = "with-summer-redis")))]
        {
            anyhow::bail!(
                "No storage backend available. Enable 'memory' or 'with-summer-redis' feature."
            );
        }
    }
}
