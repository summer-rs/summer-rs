use summer::{auto_config, App};
use summer_opendal::{Op, OpenDALPlugin};
use summer_web::extractor::Component;
use summer_web::{axum::http::StatusCode, axum::response::IntoResponse};
use summer_web::{get, post};
use summer_web::{WebConfigurator, WebPlugin};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(WebPlugin)
        .add_plugin(OpenDALPlugin)
        .run()
        .await
}

#[get("/")]
async fn index() -> impl IntoResponse {
    "Hello, OpenDAL!"
}

const FILE_NAME: &str = "test.summer";

#[get("/read")]
async fn read_file(Component(op): Component<Op>) -> impl IntoResponse {
    let b = op.exists(FILE_NAME).await.unwrap();
    if !b {
        return (StatusCode::NOT_FOUND, "File not found".to_string());
    }
    let bf = op.read_with(FILE_NAME).await.unwrap();
    (StatusCode::OK, String::from_utf8(bf.to_vec()).unwrap())
}

#[get("/info")]
async fn stat_file(Component(op): Component<Op>) -> impl IntoResponse {
    (StatusCode::OK, format!("{:?}", op.info()))
}

#[post("/write")]
async fn write_file(Component(op): Component<Op>) -> impl IntoResponse {
    match op.write(FILE_NAME, "Hello, World!").await {
        Ok(_) => (StatusCode::OK, "Write file success".to_string()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
