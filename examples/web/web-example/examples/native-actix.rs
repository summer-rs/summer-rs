use actix_web::{get, web::Data, App, HttpServer, Responder};
use anyhow::Context;
use sqlx::Row;

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let data = app::init_app().await;
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(data.clone()))
            .service(greet)
            .service(sql)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}

#[get("/")]
async fn greet() -> impl Responder {
    "Hello world!"
}

#[get("/sql")]
async fn sql(state: Data<app::AppState>) -> impl Responder {
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
