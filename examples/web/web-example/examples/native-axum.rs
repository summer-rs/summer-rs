use anyhow::Context;
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use sqlx::Row;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/sql", get(sql))
        .with_state(app::init_app().await);

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> impl IntoResponse {
    "Hello, World!"
}

async fn sql(state: State<app::AppState>) -> impl IntoResponse {
    let version: String = sqlx::query("select version() as version")
        .fetch_one(&state.db)
        .await
        .context("sqlx query failed")
        .unwrap()
        .get("version");
    version
}

mod app {
    use sqlx::{
        any::{install_default_drivers, AnyPoolOptions},
        AnyPool,
    };

    #[derive(Clone)]
    pub struct AppState {
        pub db: AnyPool,
    }

    pub async fn init_app() -> AppState {
        install_default_drivers();
        let db = AnyPoolOptions::new()
            .min_connections(10)
            .connect("postgres://postgres:xudjf23adj213@127.0.0.1:5432")
            .await
            .unwrap();
        AppState { db }
    }
}
