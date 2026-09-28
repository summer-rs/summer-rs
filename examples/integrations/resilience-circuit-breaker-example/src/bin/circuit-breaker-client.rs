use serde::Deserialize;
use std::time::Duration;
use summer::App;
use summer_resilience::{circuit_breaker, CallNotPermitted, ResiliencePlugin};
use thiserror::Error;

const PROVIDER_URL: &str = "http://127.0.0.1:18082";

#[derive(Debug, Deserialize)]
struct FxQuote {
    base: String,
    quote: String,
    rate: f64,
}

#[derive(Debug, Error)]
enum QuoteError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    CircuitOpen(#[from] CallNotPermitted),
}

#[circuit_breaker(name = "fx-quotes")]
async fn fetch_fx_quote(
    client: &reqwest::Client,
    provider_url: &str,
    base: &str,
    quote: &str,
) -> Result<FxQuote, QuoteError> {
    let response = client
        .get(format!("{provider_url}/quotes/{base}/{quote}"))
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(ResiliencePlugin)
        .build()
        .await
        .expect("application should build");

    let client = reqwest::Client::new();
    for call in 1..=2 {
        println!(
            "failed HTTP call {call}: {:?}",
            fetch_fx_quote(&client, PROVIDER_URL, "USD", "THB").await
        );
    }

    println!(
        "call rejected without HTTP traffic: {:?}",
        fetch_fx_quote(&client, PROVIDER_URL, "USD", "THB").await
    );

    tokio::time::sleep(Duration::from_millis(110)).await;
    match fetch_fx_quote(&client, PROVIDER_URL, "USD", "THB").await {
        Ok(quote) => println!(
            "half-open HTTP probe returned {}/{} rate: {}",
            quote.base, quote.quote, quote.rate
        ),
        Err(error) => eprintln!("half-open probe failed: {error}"),
    }
}
