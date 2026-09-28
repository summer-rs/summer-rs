use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use summer::{auto_config, App};
use summer_web::axum::http::StatusCode;
use summer_web::extractor::{Json, Path};
use summer_web::{get, WebConfigurator, WebPlugin};

static REQUESTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize)]
struct FxQuote {
    base: String,
    quote: String,
    rate: f64,
}

#[get("/quotes/{base}/{quote}")]
async fn quote(Path((base, quote)): Path<(String, String)>) -> Result<Json<FxQuote>, StatusCode> {
    let request = REQUESTS.fetch_add(1, Ordering::SeqCst) + 1;
    println!("provider received request {request} for {base}/{quote}");

    if request < 3 {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    } else {
        Ok(Json(FxQuote {
            base,
            quote,
            rate: 35.42,
        }))
    }
}

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new().add_plugin(WebPlugin).run().await;
}
