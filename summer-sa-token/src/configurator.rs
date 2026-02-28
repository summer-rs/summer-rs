//! Sa-Token Configurator module
//!
//! This module provides a summer Security-like configuration mechanism
//! for path-based authentication.
//!
//! # Example
//!
//! Define your security configuration in a separate file:
//!
//! ```rust,ignore
//! // config.rs
//! use summer_sa_token::{PathAuthBuilder, SaTokenConfigurator};
//!
//! pub struct SaTokenConfig;
//!
//! impl SaTokenConfigurator for SaTokenConfig {
//!     fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
//!         auth.include("/user/**")
//!             .include("/admin/**")
//!             .exclude("/login")
//!             .exclude("/public/**")
//!     }
//! }
//! ```
//!
//! Then use it in main.rs:
//!
//! ```rust,ignore
//! // main.rs
//! mod config;
//! use summer_sa_token::SaTokenAuthConfigurator;
//!
//! App::new()
//!     .add_plugin(RedisPlugin)
//!     .add_plugin(SaTokenPlugin)
//!     .add_plugin(WebPlugin)
//!     .sa_token_configure(config::SaTokenConfig)
//!     .run()
//!     .await
//! ```

use sa_token_adapter::storage::SaStorage;
use sa_token_core::router::PathAuthConfig;
use summer::app::AppBuilder;
use summer::plugin::MutableComponentRegistry;
use std::sync::Arc;

/// Trait for configuring Sa-Token path-based authentication
///
/// Implement this trait to define your security configuration,
/// similar to summer Security's configuration classes.
///
/// # Example
///
/// ```rust,ignore
/// use summer_sa_token::{PathAuthBuilder, SaTokenConfigurator};
///
/// pub struct SaTokenConfig;
///
/// impl SaTokenConfigurator for SaTokenConfig {
///     fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
///         auth.include("/api/**")
///             .include("/user/**")
///             .exclude("/login")
///             .exclude("/public/**")
///     }
/// }
/// ```
pub trait SaTokenConfigurator: Send + Sync + 'static {
    /// Configure path-based authentication rules (legacy).
    ///
    /// Prefer implementing [`SaTokenConfigurator::configure_path_auth`].
    #[deprecated(note = "use `configure_path_auth` instead")]
    fn configure(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
        auth
    }

    /// Configure path-based authentication rules.
    ///
    /// Receives a [`PathAuthBuilder`] and should return it with your rules applied.
    ///
    /// By default, this falls back to the legacy [`SaTokenConfigurator::configure`] method,
    /// so existing implementations keep working.
    fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
        #[allow(deprecated)]
        self.configure(auth)
    }

    /// Optional hook to provide a custom token storage implementation.
    ///
    /// If this returns `Some(storage)`, `summer-sa-token` will register it as a component
    /// (`Arc<dyn SaStorage>`), and the Sa-Token plugin will prefer it over built-in storages
    /// (e.g. redis/memory).
    ///
    /// Default: `None` (use built-in storage selection).
    fn configure_storage(&self, _app: &AppBuilder) -> Option<Arc<dyn SaStorage>> {
        None
    }
}

/// Builder for path-based authentication configuration
///
/// Provides a fluent API for configuring which paths require authentication.
///
/// # Supported Patterns (Ant-style)
///
/// - `/**` - Match all paths
/// - `/api/**` - Match all paths starting with `/api/`
/// - `/api/*` - Match single-level paths under `/api/` (not nested)
/// - `*.html` - Match paths ending with `.html`
/// - `/exact` - Exact match
///
/// # Example
///
/// ```rust,ignore
/// let config = PathAuthBuilder::new()
///     .include("/api/**")
///     .include("/user/**")
///     .exclude("/login")
///     .exclude("/register")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct PathAuthBuilder {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl PathAuthBuilder {
    /// Create a new PathAuthBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a path pattern that requires authentication
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// auth.include("/api/**")  // All paths under /api/ require auth
    /// ```
    pub fn include(mut self, pattern: impl Into<String>) -> Self {
        self.include.push(pattern.into());
        self
    }

    /// Add multiple path patterns that require authentication
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// auth.include_all(["/api/**", "/user/**", "/admin/**"])
    /// ```
    pub fn include_all(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.include.extend(patterns.into_iter().map(|p| p.into()));
        self
    }

    /// Add a path pattern that is excluded from authentication
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// auth.exclude("/login")  // /login doesn't require auth
    /// ```
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude.push(pattern.into());
        self
    }

    /// Add multiple path patterns that are excluded from authentication
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// auth.exclude_all(["/login", "/register", "/public/**"])
    /// ```
    pub fn exclude_all(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.exclude.extend(patterns.into_iter().map(|p| p.into()));
        self
    }

    /// Alias for `exclude` - paths that don't require authentication (permit all)
    ///
    /// Named similarly to summer Security's `permitAll()`
    pub fn permit_all(self, pattern: impl Into<String>) -> Self {
        self.exclude(pattern)
    }

    /// Alias for `include` - paths that require authentication
    ///
    /// Named similarly to summer Security's `authenticated()`
    pub fn authenticated(self, pattern: impl Into<String>) -> Self {
        self.include(pattern)
    }

    /// Check if any include patterns are configured
    pub fn is_configured(&self) -> bool {
        !self.include.is_empty()
    }

    /// Build the final PathAuthConfig
    pub fn build(self) -> PathAuthConfig {
        PathAuthConfig::new()
            .include(self.include)
            .exclude(self.exclude)
    }

    /// Merge another builder's rules into this one
    pub fn merge(mut self, other: PathAuthBuilder) -> Self {
        self.include.extend(other.include);
        self.exclude.extend(other.exclude);
        self
    }
}

// ============================================================================
// SaTokenAuthConfigurator - Fluent API for configuring path-based auth
// ============================================================================

/// Trait for configuring Sa-Token path-based authentication via AppBuilder
///
/// This provides a fluent API to configure authentication directly on the App.
///
/// # Example
///
/// Define your security configuration in a separate file:
///
/// ```rust,ignore
/// // config.rs
/// use summer_sa_token::{PathAuthBuilder, SaTokenConfigurator};
///
/// pub struct SaTokenConfig;
///
/// impl SaTokenConfigurator for SaTokenConfig {
///     fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
///         auth.include("/user/**")
///             .include("/admin/**")
///             .include("/api/**")
///             .exclude("/")
///             .exclude("/login")
///             .exclude("/public/**")
///             .exclude("/api/health")
///     }
/// }
/// ```
///
/// Then use it in main.rs:
///
/// ```rust,ignore
/// // main.rs
/// mod config;
/// use summer_sa_token::SaTokenAuthConfigurator;
///
/// App::new()
///     .add_plugin(RedisPlugin)
///     .add_plugin(SaTokenPlugin)
///     .add_plugin(WebPlugin)
///     .sa_token_configure(config::SaTokenConfig)
///     .run()
///     .await
/// ```
pub trait SaTokenAuthConfigurator {
    /// Configure path-based authentication rules using a SaTokenConfigurator implementation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// app.sa_token_configure(config::SaTokenConfig);
    /// ```
    #[deprecated(note = "use `sa_token_configure` instead")]
    fn sa_token_auth<C>(&mut self, configurator: C) -> &mut Self
    where
        C: SaTokenConfigurator;

    /// Configure path-based authentication rules using a SaTokenConfigurator implementation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// app.sa_token_configure(config::SaTokenConfig);
    /// ```
    fn sa_token_configure<C>(&mut self, configurator: C) -> &mut Self
    where
        C: SaTokenConfigurator;
}

impl SaTokenAuthConfigurator for AppBuilder {
    #[allow(deprecated)]
    fn sa_token_auth<C>(&mut self, configurator: C) -> &mut Self
    where
        C: SaTokenConfigurator,
    {
        self.sa_token_configure(configurator)
    }

    fn sa_token_configure<C>(&mut self, configurator: C) -> &mut Self
    where
        C: SaTokenConfigurator,
    {
        if let Some(storage) = configurator.configure_storage(self) {
            self.add_component(crate::custom_storage::SaTokenStorage::new(storage));
        }

        let builder = configurator.configure_path_auth(PathAuthBuilder::new());
        if builder.is_configured() {
            let config = builder.build();
            self.add_component(config)
        } else {
            self
        }
    }
}

impl SaTokenConfigurator for PathAuthBuilder {
    fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
        auth.merge(self.clone())
    }
}
