use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use summer::config::Configurable;

summer::submit_config_schema!("resilience", ResilienceConfig);

/// Resilience policy configuration.
#[derive(Debug, Default, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "resilience"]
pub struct ResilienceConfig {
    /// Named retry policies.
    #[serde(default)]
    pub retry: RetryPoliciesConfig,

    /// Named circuit breaker policies.
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerPoliciesConfig,
}

/// Collection of named retry policy instances.
#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub struct RetryPoliciesConfig {
    /// Policies keyed by the name used in `#[retry(name = "...")]`.
    #[serde(default)]
    pub instances: HashMap<String, RetryConfig>,
}

/// Collection of named circuit breaker policy instances.
#[derive(Debug, Default, Clone, JsonSchema, Deserialize)]
pub struct CircuitBreakerPoliciesConfig {
    /// Policies keyed by the name used in `#[circuit_breaker(name = "...")]`.
    #[serde(default)]
    pub instances: HashMap<String, CircuitBreakerConfig>,
}

/// Configuration for one circuit breaker policy.
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Consecutive failures required to open the circuit.
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// Time spent open before a call can probe the dependency, in milliseconds.
    #[serde(default = "default_wait_duration_in_open_state")]
    pub wait_duration_in_open_state: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: default_failure_threshold(),
            wait_duration_in_open_state: default_wait_duration_in_open_state(),
        }
    }
}

/// Configuration for one retry policy.
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct RetryConfig {
    /// Total number of attempts, including the initial call.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Delay before the first retry, in milliseconds.
    #[serde(default = "default_wait_duration")]
    pub wait_duration: u64,

    /// Whether the delay increases after each failed attempt.
    #[serde(default)]
    pub enable_exponential_backoff: bool,

    /// Multiplier used when exponential backoff is enabled.
    #[serde(default = "default_exponential_backoff_multiplier")]
    pub exponential_backoff_multiplier: f64,

    /// Maximum base retry delay before randomization, in milliseconds.
    pub exponential_max_wait_duration: Option<u64>,

    /// Whether retry delays are randomized.
    #[serde(default)]
    pub enable_randomized_wait: bool,

    /// Randomization range around the calculated delay.
    #[serde(default = "default_randomized_wait_factor")]
    pub randomized_wait_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            wait_duration: default_wait_duration(),
            enable_exponential_backoff: false,
            exponential_backoff_multiplier: default_exponential_backoff_multiplier(),
            exponential_max_wait_duration: None,
            enable_randomized_wait: false,
            randomized_wait_factor: default_randomized_wait_factor(),
        }
    }
}

const fn default_max_attempts() -> u32 {
    3
}

const fn default_wait_duration() -> u64 {
    500
}

const fn default_exponential_backoff_multiplier() -> f64 {
    2.0
}

const fn default_randomized_wait_factor() -> f64 {
    0.5
}

const fn default_failure_threshold() -> u32 {
    5
}

const fn default_wait_duration_in_open_state() -> u64 {
    60_000
}
