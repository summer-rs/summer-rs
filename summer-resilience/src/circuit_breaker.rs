use crate::config::CircuitBreakerConfig;
use failsafe::futures::CircuitBreaker as _;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

const CLOSED: u8 = 0;
const OPEN: u8 = 1;
const HALF_OPEN: u8 = 2;

/// The externally visible state of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
struct CircuitInstrument {
    name: Arc<str>,
    state: Arc<AtomicU8>,
}

impl failsafe::Instrument for CircuitInstrument {
    fn on_call_rejected(&self) {
        tracing::debug!(circuit_breaker.name = %self.name, "circuit breaker rejected a call");
    }

    fn on_open(&self) {
        self.state.store(OPEN, Ordering::Release);
        tracing::debug!(circuit_breaker.name = %self.name, "circuit breaker opened");
    }

    fn on_half_open(&self) {
        self.state.store(HALF_OPEN, Ordering::Release);
        tracing::debug!(circuit_breaker.name = %self.name, "circuit breaker entered half-open state");
    }

    fn on_closed(&self) {
        self.state.store(CLOSED, Ordering::Release);
        tracing::debug!(circuit_breaker.name = %self.name, "circuit breaker closed");
    }
}

type FailsafeCircuitBreaker = failsafe::StateMachine<
    failsafe::failure_policy::ConsecutiveFailures<failsafe::backoff::Constant>,
    CircuitInstrument,
>;

/// One named Failsafe circuit breaker shared between asynchronous calls.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    name: Arc<str>,
    inner: FailsafeCircuitBreaker,
    state: Arc<AtomicU8>,
}

impl CircuitBreaker {
    fn try_new(
        name: String,
        config: CircuitBreakerConfig,
    ) -> Result<Self, CircuitBreakerConfigError> {
        if config.failure_threshold == 0 {
            return Err(CircuitBreakerConfigError::ZeroFailureThreshold);
        }
        if config.wait_duration_in_open_state == 0 {
            return Err(CircuitBreakerConfigError::ZeroOpenStateDuration);
        }

        let name: Arc<str> = Arc::from(name);
        let state = Arc::new(AtomicU8::new(CLOSED));
        let instrument = CircuitInstrument {
            name: name.clone(),
            state: state.clone(),
        };
        let backoff =
            failsafe::backoff::constant(Duration::from_millis(config.wait_duration_in_open_state));
        let policy =
            failsafe::failure_policy::consecutive_failures(config.failure_threshold, backoff);
        let inner = failsafe::Config::new()
            .failure_policy(policy)
            .instrument(instrument)
            .build();

        Ok(Self { name, inner, state })
    }

    /// Returns the last state reported by Failsafe instrumentation.
    pub async fn state(&self) -> CircuitBreakerState {
        match self.state.load(Ordering::Acquire) {
            CLOSED => CircuitBreakerState::Closed,
            OPEN => CircuitBreakerState::Open,
            HALF_OPEN => CircuitBreakerState::HalfOpen,
            _ => unreachable!("invalid circuit breaker state"),
        }
    }
}

/// Invalid circuit breaker policy configuration.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CircuitBreakerConfigError {
    #[error("failure_threshold must be at least 1")]
    ZeroFailureThreshold,
    #[error("wait_duration_in_open_state must be at least 1 millisecond")]
    ZeroOpenStateDuration,
}

/// Error returned when Failsafe rejects a call while the circuit is open.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("circuit breaker `{name}` does not permit calls")]
pub struct CallNotPermitted {
    name: String,
}

impl CallNotPermitted {
    /// Name of the circuit breaker that rejected the call.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Result error from the programmatic circuit breaker API.
#[derive(Debug, Error)]
pub enum CircuitBreakerError<E> {
    #[error(transparent)]
    CallNotPermitted(#[from] CallNotPermitted),
    #[error("guarded operation failed")]
    Operation(E),
}

/// Executes one asynchronous operation through Failsafe.
pub async fn execute<F, Fut, T, E, P>(
    circuit_breaker: CircuitBreaker,
    operation: F,
    record_failure: P,
) -> Result<T, CircuitBreakerError<E>>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: Fn(&E) -> bool,
{
    match circuit_breaker
        .inner
        .call_with(record_failure, operation())
        .await
    {
        Ok(value) => Ok(value),
        Err(failsafe::Error::Inner(error)) => Err(CircuitBreakerError::Operation(error)),
        Err(failsafe::Error::Rejected) => Err(CallNotPermitted {
            name: circuit_breaker.name.to_string(),
        }
        .into()),
    }
}

/// Registry of named circuit breakers.
#[derive(Debug, Clone, Default)]
pub struct CircuitBreakerRegistry {
    circuit_breakers: HashMap<String, CircuitBreaker>,
}

impl CircuitBreakerRegistry {
    pub(crate) fn from_configs(
        configs: HashMap<String, CircuitBreakerConfig>,
    ) -> Result<Self, NamedCircuitBreakerConfigError> {
        let circuit_breakers = configs
            .into_iter()
            .map(
                |(name, config)| match CircuitBreaker::try_new(name.clone(), config) {
                    Ok(circuit_breaker) => Ok((name, circuit_breaker)),
                    Err(source) => Err(NamedCircuitBreakerConfigError { name, source }),
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Self { circuit_breakers })
    }

    /// Gets a named circuit breaker.
    pub fn get(&self, name: &str) -> Option<CircuitBreaker> {
        self.circuit_breakers.get(name).cloned()
    }
}

#[derive(Debug, Error)]
#[error("invalid circuit breaker policy `{name}`: {source}")]
pub(crate) struct NamedCircuitBreakerConfigError {
    name: String,
    #[source]
    source: CircuitBreakerConfigError,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker(config: CircuitBreakerConfig) -> CircuitBreaker {
        CircuitBreaker::try_new("test".into(), config).unwrap()
    }

    #[tokio::test]
    async fn opens_after_the_configured_consecutive_failures() {
        let breaker = breaker(CircuitBreakerConfig {
            failure_threshold: 2,
            ..CircuitBreakerConfig::default()
        });

        for _ in 0..2 {
            let result = execute(
                breaker.clone(),
                || async { Err::<(), _>("failed") },
                |_| true,
            )
            .await;
            assert!(matches!(
                result,
                Err(CircuitBreakerError::Operation("failed"))
            ));
        }

        assert_eq!(breaker.state().await, CircuitBreakerState::Open);
        let result = execute(breaker, || async { Ok::<_, &str>(()) }, |_| true).await;
        assert!(matches!(
            result,
            Err(CircuitBreakerError::CallNotPermitted(_))
        ));
    }

    #[tokio::test]
    async fn successful_half_open_probe_closes_the_circuit() {
        let breaker = breaker(CircuitBreakerConfig {
            failure_threshold: 1,
            wait_duration_in_open_state: 1,
        });
        let _ = execute(
            breaker.clone(),
            || async { Err::<(), _>("failed") },
            |_| true,
        )
        .await;
        tokio::time::sleep(Duration::from_millis(2)).await;

        let result = execute(breaker.clone(), || async { Ok::<_, &str>(42) }, |_| true).await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(breaker.state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn ignored_errors_do_not_open_the_circuit() {
        let breaker = breaker(CircuitBreakerConfig {
            failure_threshold: 1,
            ..CircuitBreakerConfig::default()
        });

        let result = execute(
            breaker.clone(),
            || async { Err::<(), _>("ignored") },
            |_| false,
        )
        .await;

        assert!(matches!(
            result,
            Err(CircuitBreakerError::Operation("ignored"))
        ));
        assert_eq!(breaker.state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn cancelled_half_open_probe_does_not_block_the_next_probe() {
        let breaker = breaker(CircuitBreakerConfig {
            failure_threshold: 1,
            wait_duration_in_open_state: 1,
        });
        let _ = execute(
            breaker.clone(),
            || async { Err::<(), _>("failed") },
            |_| true,
        )
        .await;
        tokio::time::sleep(Duration::from_millis(2)).await;

        let cancelled = tokio::time::timeout(
            Duration::from_millis(1),
            execute(
                breaker.clone(),
                || async {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Ok::<_, &str>(())
                },
                |_| true,
            ),
        )
        .await;
        assert!(cancelled.is_err());
        assert_eq!(breaker.state().await, CircuitBreakerState::HalfOpen);

        let result = execute(breaker.clone(), || async { Ok::<_, &str>(42) }, |_| true).await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(breaker.state().await, CircuitBreakerState::Closed);
    }

    #[test]
    fn rejects_a_zero_failure_threshold() {
        let result = CircuitBreaker::try_new(
            "test".into(),
            CircuitBreakerConfig {
                failure_threshold: 0,
                ..CircuitBreakerConfig::default()
            },
        );
        assert_eq!(
            result.unwrap_err(),
            CircuitBreakerConfigError::ZeroFailureThreshold
        );
    }
}
