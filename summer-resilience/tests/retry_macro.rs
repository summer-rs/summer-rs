use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use summer::App;
use summer_resilience::{retry, ResiliencePlugin};

#[retry(name = "unstable")]
async fn eventually_succeeds(
    attempts: Arc<AtomicUsize>,
    succeed_on: usize,
) -> Result<usize, &'static str> {
    let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
    if attempt < succeed_on {
        Err("temporary")
    } else {
        Ok(attempt)
    }
}

fn only_temporary(error: &&'static str) -> bool {
    *error == "temporary"
}

#[retry(name = "classified", retry_if = only_temporary)]
async fn classified_error(attempts: Arc<AtomicUsize>) -> Result<(), &'static str> {
    attempts.fetch_add(1, Ordering::SeqCst);
    Err("permanent")
}

struct RetryingClient {
    attempts: Arc<AtomicUsize>,
}

impl RetryingClient {
    #[retry(name = "method")]
    async fn call(&self) -> Result<usize, &'static str> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt < 2 {
            Err("temporary")
        } else {
            Ok(attempt)
        }
    }
}

#[tokio::test]
async fn macro_uses_named_policy_and_clones_arguments() {
    App::new()
        .use_config_str(
            r#"
            [resilience.retry.instances.unstable]
            max_attempts = 3
            wait_duration = 0

            [resilience.retry.instances.classified]
            max_attempts = 3
            wait_duration = 0

            [resilience.retry.instances.method]
            max_attempts = 2
            wait_duration = 0
            "#,
        )
        .add_plugin(ResiliencePlugin)
        .build()
        .await
        .expect("app should build");

    let attempts = Arc::new(AtomicUsize::new(0));
    assert_eq!(eventually_succeeds(attempts.clone(), 3).await, Ok(3));
    assert_eq!(attempts.load(Ordering::SeqCst), 3);

    let attempts = Arc::new(AtomicUsize::new(0));
    assert_eq!(classified_error(attempts.clone()).await, Err("permanent"));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    let client = RetryingClient {
        attempts: Arc::new(AtomicUsize::new(0)),
    };
    assert_eq!(client.call().await, Ok(2));
    assert_eq!(client.attempts.load(Ordering::SeqCst), 2);
}
