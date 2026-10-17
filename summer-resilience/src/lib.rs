//! Retry and other resilience policies for summer-rs applications.
#![doc = include_str!("../README.md")]
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;
pub mod retry;

pub use config::{ResilienceConfig, RetryConfig};
pub use retry::{execute as execute_retry, RetryConfigError, RetryPolicy, RetryRegistry};
pub use summer_macros::retry;

use summer::config::ConfigRegistry;
use summer::plugin::MutableComponentRegistry;
use summer::{app::AppBuilder, async_trait, plugin::Plugin};

/// Loads resilience configuration and registers the policy registries.
pub struct ResiliencePlugin;

#[async_trait]
impl Plugin for ResiliencePlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<ResilienceConfig>()
            .expect("resilience plugin config load failed");
        let retry_registry = RetryRegistry::from_configs(config.retry.instances)
            .expect("resilience retry config validation failed");
        app.add_component(retry_registry);
    }
}
