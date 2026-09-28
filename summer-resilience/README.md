[![crates.io](https://img.shields.io/crates/v/summer-resilience.svg)](https://crates.io/crates/summer-resilience)
[![Documentation](https://docs.rs/summer-resilience/badge.svg)](https://docs.rs/summer-resilience)

`summer-resilience` provides named resilience policies for asynchronous application operations.

## Retry configuration

```toml
[resilience.retry.instances.inventory]
max_attempts = 3
wait_duration = 100
enable_exponential_backoff = true
exponential_backoff_multiplier = 2.0
exponential_max_wait_duration = 2000
enable_randomized_wait = true
randomized_wait_factor = 0.5
```

Durations are expressed in milliseconds. `max_attempts` includes the initial call.

## Retry macro

Add `ResiliencePlugin` to the application, then refer to a configured policy by name:

```rust,ignore
use summer::App;
use summer_resilience::{retry, ResiliencePlugin};

#[retry(name = "inventory")]
async fn load_inventory(id: String) -> Result<String, InventoryError> {
    inventory_client().load(id).await
}

App::new().add_plugin(ResiliencePlugin);
```

All errors are retried by default. A predicate can classify errors that are safe to retry:

```rust,ignore
#[retry(name = "inventory", retry_if = is_transient)]
async fn load_inventory(id: String) -> Result<String, InventoryError> {
    inventory_client().load(id).await
}

fn is_transient(error: &InventoryError) -> bool {
    error.is_timeout() || error.is_unavailable()
}
```

The operation's final error is returned unchanged. Retried arguments are cloned before every attempt,
so owned function parameters must implement `Clone`. Mutable receivers and destructured parameter
patterns are not supported.

Retrying can repeat side effects. Operations such as payments, writes, and message publication need
their own idempotency guarantees.

## Circuit breaker configuration

Circuit breakers use Failsafe's consecutive-failure policy and state machine:

```toml
[resilience.circuit_breaker.instances.inventory]
failure_threshold = 5
wait_duration_in_open_state = 30000
```

Guard an asynchronous function with the configured instance:

```rust,ignore
use summer_resilience::{circuit_breaker, CallNotPermitted};

#[derive(Debug, thiserror::Error)]
enum InventoryError {
    #[error("inventory dependency failed")]
    Dependency,
    #[error(transparent)]
    CircuitOpen(#[from] CallNotPermitted),
}

#[circuit_breaker(name = "inventory")]
async fn load_inventory(id: String) -> Result<String, InventoryError> {
    inventory_client().load(id).await
}
```

The function error type must implement `From<CallNotPermitted>`, making rejected calls explicit.
Operation errors count as failures by default. Use `record_failure = predicate_name` to classify
which errors should affect the circuit breaker.
