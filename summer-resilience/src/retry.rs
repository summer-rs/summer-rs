use crate::config::RetryConfig;
use backoff::backoff::Backoff;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use thiserror::Error;

/// A validated retry policy used at runtime.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    max_attempts: u32,
    wait_duration: Duration,
    exponential_backoff_multiplier: f64,
    max_wait_duration: Duration,
    randomized_wait_factor: f64,
}

impl RetryPolicy {
    /// Total number of calls, including the initial attempt.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    fn backoff(&self) -> ExponentialBackoff {
        let mut builder = ExponentialBackoffBuilder::new();
        builder
            .with_initial_interval(self.wait_duration)
            .with_multiplier(self.exponential_backoff_multiplier)
            .with_max_interval(self.max_wait_duration)
            .with_randomization_factor(self.randomized_wait_factor)
            .with_max_elapsed_time(None);
        builder.build()
    }
}

impl TryFrom<RetryConfig> for RetryPolicy {
    type Error = RetryConfigError;

    fn try_from(config: RetryConfig) -> Result<Self, Self::Error> {
        if config.max_attempts == 0 {
            return Err(RetryConfigError::ZeroMaxAttempts);
        }
        if config.enable_exponential_backoff
            && (!config.exponential_backoff_multiplier.is_finite()
                || config.exponential_backoff_multiplier < 1.0)
        {
            return Err(RetryConfigError::InvalidMultiplier(
                config.exponential_backoff_multiplier,
            ));
        }
        if config.enable_randomized_wait
            && (!config.randomized_wait_factor.is_finite()
                || !(0.0..=1.0).contains(&config.randomized_wait_factor))
        {
            return Err(RetryConfigError::InvalidRandomizedWaitFactor(
                config.randomized_wait_factor,
            ));
        }

        let wait_duration = Duration::from_millis(config.wait_duration);
        let max_wait_duration = if config.enable_exponential_backoff {
            config
                .exponential_max_wait_duration
                .map(Duration::from_millis)
                .unwrap_or_else(|| Duration::from_millis(60_000))
        } else {
            wait_duration
        };
        if config.enable_exponential_backoff && max_wait_duration < wait_duration {
            return Err(RetryConfigError::MaxWaitBelowInitialWait);
        }

        Ok(Self {
            max_attempts: config.max_attempts,
            wait_duration,
            exponential_backoff_multiplier: if config.enable_exponential_backoff {
                config.exponential_backoff_multiplier
            } else {
                1.0
            },
            max_wait_duration,
            randomized_wait_factor: if config.enable_randomized_wait {
                config.randomized_wait_factor
            } else {
                0.0
            },
        })
    }
}

/// Invalid retry policy configuration.
#[derive(Debug, Error, PartialEq)]
pub enum RetryConfigError {
    /// `max_attempts` must include at least the initial call.
    #[error("max_attempts must be at least 1")]
    ZeroMaxAttempts,

    /// Exponential growth cannot use a multiplier below one or a non-finite value.
    #[error("exponential_backoff_multiplier must be finite and at least 1, got {0}")]
    InvalidMultiplier(f64),

    /// Randomization is expressed as a fraction from zero to one.
    #[error("randomized_wait_factor must be between 0 and 1, got {0}")]
    InvalidRandomizedWaitFactor(f64),

    /// A maximum delay cannot be shorter than the initial delay.
    #[error("exponential_max_wait_duration cannot be less than wait_duration")]
    MaxWaitBelowInitialWait,
}

/// Registry of validated, named retry policies.
#[derive(Debug, Clone, Default)]
pub struct RetryRegistry {
    policies: HashMap<String, RetryPolicy>,
}

impl RetryRegistry {
    pub(crate) fn from_configs(
        configs: HashMap<String, RetryConfig>,
    ) -> Result<Self, NamedRetryConfigError> {
        let policies = configs
            .into_iter()
            .map(|(name, config)| {
                RetryPolicy::try_from(config)
                    .map(|policy| (name.clone(), policy))
                    .map_err(|source| NamedRetryConfigError { name, source })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { policies })
    }

    /// Gets a named retry policy.
    pub fn get(&self, name: &str) -> Option<RetryPolicy> {
        self.policies.get(name).cloned()
    }
}

/// Invalid configuration associated with a named retry policy.
#[derive(Debug, Error)]
#[error("invalid retry policy `{name}`: {source}")]
pub struct NamedRetryConfigError {
    name: String,
    #[source]
    source: RetryConfigError,
}

/// Executes an asynchronous operation according to a retry policy.
///
/// `retry_if` classifies operation errors. Returning `false` stops immediately.
/// The final operation error is returned unchanged.
pub async fn execute<F, Fut, T, E, P>(
    name: &str,
    policy: RetryPolicy,
    mut operation: F,
    retry_if: P,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: Fn(&E) -> bool,
{
    let mut attempt = 1;
    let mut backoff = policy.backoff();

    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) if attempt < policy.max_attempts && retry_if(&error) => {
                let delay = backoff
                    .next_backoff()
                    .expect("retry policy has no elapsed-time limit");
                tracing::debug!(
                    retry.name = name,
                    retry.attempt = attempt,
                    retry.max_attempts = policy.max_attempts,
                    retry.delay_ms = delay.as_millis(),
                    "retrying operation after failure"
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn immediate_policy(max_attempts: u32) -> RetryPolicy {
        RetryPolicy::try_from(RetryConfig {
            max_attempts,
            wait_duration: 0,
            ..RetryConfig::default()
        })
        .unwrap()
    }

    #[tokio::test]
    async fn retries_until_the_operation_succeeds() {
        let attempts = AtomicUsize::new(0);
        let result = execute(
            "test",
            immediate_policy(3),
            || async {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt < 3 {
                    Err("temporary")
                } else {
                    Ok(42)
                }
            },
            |_| true,
        )
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stops_when_the_error_is_not_retryable() {
        let attempts = AtomicUsize::new(0);
        let result = execute(
            "test",
            immediate_policy(3),
            || async {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>("permanent")
            },
            |_| false,
        )
        .await;

        assert_eq!(result, Err("permanent"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn rejects_invalid_configuration() {
        let error = RetryPolicy::try_from(RetryConfig {
            max_attempts: 0,
            ..RetryConfig::default()
        })
        .unwrap_err();

        assert_eq!(error, RetryConfigError::ZeroMaxAttempts);
    }

    #[test]
    fn exponential_backoff_grows_until_the_configured_cap() {
        let policy = RetryPolicy::try_from(RetryConfig {
            wait_duration: 10,
            enable_exponential_backoff: true,
            exponential_backoff_multiplier: 2.0,
            exponential_max_wait_duration: Some(25),
            ..RetryConfig::default()
        })
        .unwrap();
        let mut backoff = policy.backoff();

        assert_eq!(backoff.next_backoff(), Some(Duration::from_millis(10)));
        assert_eq!(backoff.next_backoff(), Some(Duration::from_millis(20)));
        assert_eq!(backoff.next_backoff(), Some(Duration::from_millis(25)));
    }
}
