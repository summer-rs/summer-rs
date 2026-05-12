use anyhow::Context;
use ntex::web::{self, types::State, Responder};
use sqlx::Row;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let data = app::init_app().await;
    web::HttpServer::new(move || {
        web::App::new()
            .state(data.clone())
            .service(index)
            .service(sql)
    })
    .bind(("127.0.0.1", 8082))?
    .run()
    .await
}

#[web::get("/")]
async fn index() -> impl web::Responder {
    "Hello, World!"
}

#[web::get("/sql")]
async fn sql(state: State<app::AppState>) -> impl Responder {
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
