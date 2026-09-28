use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use summer::App;
use summer_resilience::{circuit_breaker, CallNotPermitted, ResiliencePlugin};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
enum ServiceError {
    #[error("dependency failed")]
    Dependency,
    #[error(transparent)]
    CircuitOpen(#[from] CallNotPermitted),
}

#[circuit_breaker(name = "backend")]
async fn call_backend(attempts: Arc<AtomicUsize>) -> Result<(), ServiceError> {
    attempts.fetch_add(1, Ordering::SeqCst);
    Err(ServiceError::Dependency)
}

#[tokio::test]
async fn macro_returns_a_typed_error_when_the_circuit_is_open() {
    App::new()
        .use_config_str(
            r#"
            [resilience.circuit_breaker.instances.backend]
            failure_threshold = 1
            wait_duration_in_open_state = 60000
            "#,
        )
        .add_plugin(ResiliencePlugin)
        .build()
        .await
        .expect("app should build");

    let attempts = Arc::new(AtomicUsize::new(0));
    assert_eq!(
        call_backend(attempts.clone()).await,
        Err(ServiceError::Dependency)
    );

    let error = call_backend(attempts.clone()).await.unwrap_err();
    assert!(matches!(error, ServiceError::CircuitOpen(_)));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}
