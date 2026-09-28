//! Retry and other resilience policies for summer-rs applications.
#![doc = include_str!("../README.md")]
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod circuit_breaker;
pub mod config;
pub mod retry;

pub use circuit_breaker::{
    execute as execute_circuit_breaker, CallNotPermitted, CircuitBreaker,
    CircuitBreakerConfigError, CircuitBreakerError, CircuitBreakerRegistry, CircuitBreakerState,
};
pub use config::{CircuitBreakerConfig, ResilienceConfig, RetryConfig};
pub use retry::{execute as execute_retry, RetryConfigError, RetryPolicy, RetryRegistry};
pub use summer_macros::{circuit_breaker, retry};

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
        let circuit_breaker_registry =
            CircuitBreakerRegistry::from_configs(config.circuit_breaker.instances)
                .expect("resilience circuit breaker config validation failed");
        app.add_component(retry_registry);
        app.add_component(circuit_breaker_registry);
    }
}
