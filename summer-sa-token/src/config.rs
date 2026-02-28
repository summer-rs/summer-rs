//! Sa-Token configuration module
//!
//! This module defines the configuration for summer-sa-token plugin.

use serde::{Deserialize, Serialize};
use summer::config::Configurable;
use schemars::JsonSchema;
// Re-export CoreConfig from upstream
pub use sa_token_core::config::SaTokenConfig as CoreConfig;

summer::submit_config_schema!("sa-token", SaTokenConfig);

/// Token style for summer-sa-token
///
/// This is a local wrapper around the upstream TokenStyle to support JsonSchema
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum TokenStyle {
    /// UUID style
    Uuid,
    /// Simple UUID (without hyphens)
    SimpleUuid,
    /// 32-character random string
    Random32,
    /// 64-character random string
    Random64,
    /// 128-character random string
    Random128,
    /// JWT style (JSON Web Token)
    Jwt,
    /// Hash style (SHA256 hash)
    Hash,
    /// Timestamp style (millisecond timestamp + random)
    Timestamp,
    /// Tik style (short 8-character token)
    Tik,
}

impl From<TokenStyle> for sa_token_core::config::TokenStyle {
    fn from(style: TokenStyle) -> Self {
        match style {
            TokenStyle::Uuid => sa_token_core::config::TokenStyle::Uuid,
            TokenStyle::SimpleUuid => sa_token_core::config::TokenStyle::SimpleUuid,
            TokenStyle::Random32 => sa_token_core::config::TokenStyle::Random32,
            TokenStyle::Random64 => sa_token_core::config::TokenStyle::Random64,
            TokenStyle::Random128 => sa_token_core::config::TokenStyle::Random128,
            TokenStyle::Jwt => sa_token_core::config::TokenStyle::Jwt,
            TokenStyle::Hash => sa_token_core::config::TokenStyle::Hash,
            TokenStyle::Timestamp => sa_token_core::config::TokenStyle::Timestamp,
            TokenStyle::Tik => sa_token_core::config::TokenStyle::Tik,
        }
    }
}

/// Sa-Token configuration for summer-rs
///
/// Most fields have sensible defaults.
///
/// # Example
///
/// ```toml
/// [sa-token]
/// token_name = "Authorization"
/// timeout = 86400
/// auto_renew = true
/// ```
#[derive(Debug, Configurable, Clone, Deserialize, JsonSchema)]
#[config_prefix = "sa-token"]
pub struct SaTokenConfig {
    /// Token name (key in header or cookie)
    /// Default: "Authorization"
    #[serde(default = "default_token_name")]
    pub token_name: String,

    /// Token timeout in seconds, -1 means permanent
    /// Default: 2592000 (30 days)
    #[serde(default = "default_timeout")]
    pub timeout: i64,

    /// Token active timeout in seconds, -1 means no limit
    /// Default: -1
    #[serde(default = "default_active_timeout")]
    pub active_timeout: i64,

    /// Enable auto renew
    /// Default: false
    #[serde(default)]
    pub auto_renew: bool,

    /// Allow concurrent login for same account
    /// Default: true
    #[serde(default = "default_true")]
    pub is_concurrent: bool,

    /// Share token when multiple logins
    /// Default: true
    #[serde(default = "default_true")]
    pub is_share: bool,

    /// Token style
    /// Default: Uuid
    #[serde(default = "default_token_style")]
    pub token_style: TokenStyle,

    /// Enable logging
    /// Default: false
    #[serde(default)]
    pub is_log: bool,

    /// Read token from cookie
    /// Default: true
    #[serde(default = "default_true")]
    pub is_read_cookie: bool,

    /// Read token from header
    /// Default: true
    #[serde(default = "default_true")]
    pub is_read_header: bool,

    /// Read token from body
    /// Default: false
    #[serde(default)]
    pub is_read_body: bool,

    /// Token prefix (e.g., "Bearer ")
    #[serde(default)]
    pub token_prefix: Option<String>,

    /// JWT secret key
    #[serde(default)]
    pub jwt_secret_key: Option<String>,

    /// JWT algorithm
    /// Default: "HS256"
    #[serde(default = "default_jwt_algorithm")]
    pub jwt_algorithm: Option<String>,

    /// JWT issuer
    #[serde(default)]
    pub jwt_issuer: Option<String>,

    /// JWT audience
    #[serde(default)]
    pub jwt_audience: Option<String>,

    /// Enable nonce for replay attack prevention
    /// Default: false
    #[serde(default)]
    pub enable_nonce: bool,

    /// Nonce timeout in seconds, -1 means use token timeout
    /// Default: -1
    #[serde(default = "default_nonce_timeout")]
    pub nonce_timeout: i64,

    /// Enable refresh token
    /// Default: false
    #[serde(default)]
    pub enable_refresh_token: bool,

    /// Refresh token timeout in seconds
    /// Default: 604800 (7 days)
    #[serde(default = "default_refresh_token_timeout")]
    pub refresh_token_timeout: i64,
}

impl Default for SaTokenConfig {
    fn default() -> Self {
        Self {
            token_name: default_token_name(),
            timeout: default_timeout(),
            active_timeout: default_active_timeout(),
            auto_renew: false,
            is_concurrent: true,
            is_share: true,
            token_style: TokenStyle::Uuid,
            is_log: false,
            is_read_cookie: true,
            is_read_header: true,
            is_read_body: false,
            token_prefix: None,
            jwt_secret_key: None,
            jwt_algorithm: default_jwt_algorithm(),
            jwt_issuer: None,
            jwt_audience: None,
            enable_nonce: false,
            nonce_timeout: default_nonce_timeout(),
            enable_refresh_token: false,
            refresh_token_timeout: default_refresh_token_timeout(),
        }
    }
}

impl From<SaTokenConfig> for CoreConfig {
    fn from(config: SaTokenConfig) -> Self {
        CoreConfig {
            token_name: config.token_name,
            timeout: config.timeout,
            active_timeout: config.active_timeout,
            auto_renew: config.auto_renew,
            is_concurrent: config.is_concurrent,
            is_share: config.is_share,
            token_style: config.token_style.into(),
            is_log: config.is_log,
            is_read_cookie: config.is_read_cookie,
            is_read_header: config.is_read_header,
            is_read_body: config.is_read_body,
            token_prefix: config.token_prefix,
            jwt_secret_key: config.jwt_secret_key,
            jwt_algorithm: config.jwt_algorithm,
            jwt_issuer: config.jwt_issuer,
            jwt_audience: config.jwt_audience,
            enable_nonce: config.enable_nonce,
            nonce_timeout: config.nonce_timeout,
            enable_refresh_token: config.enable_refresh_token,
            refresh_token_timeout: config.refresh_token_timeout,
        }
    }
}

// Default value functions
fn default_token_name() -> String {
    "Authorization".to_string()
}

fn default_timeout() -> i64 {
    2592000 // 30 days
}

fn default_active_timeout() -> i64 {
    -1
}

fn default_true() -> bool {
    true
}

fn default_jwt_algorithm() -> Option<String> {
    Some("HS256".to_string())
}

fn default_nonce_timeout() -> i64 {
    -1
}

fn default_refresh_token_timeout() -> i64 {
    604800 // 7 days
}

fn default_token_style() -> TokenStyle {
    TokenStyle::Uuid
}
