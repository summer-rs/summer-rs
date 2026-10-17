use serde::Deserialize;
use summer::App;
use summer_resilience::{retry, ResiliencePlugin};
use thiserror::Error;

const PROVIDER_URL: &str = "http://127.0.0.1:18081";

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
}

#[retry(name = "fx-quotes")]
async fn fetch_fx_quote(
    client: reqwest::Client,
    provider_url: String,
    base: String,
    quote: String,
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

    match fetch_fx_quote(
        reqwest::Client::new(),
        PROVIDER_URL.to_owned(),
        "USD".to_owned(),
        "THB".to_owned(),
    )
    .await
    {
        Ok(quote) => println!("{}/{} rate: {}", quote.base, quote.quote, quote.rate),
        Err(error) => eprintln!("quote failed: {error}"),
    }
}
