use anyhow::Context;
use summer::{auto_config, App};
use summer_postgres::{PgPlugin, Postgres};
use summer_web::get;
use summer_web::{
    axum::response::{IntoResponse, Json},
    error::Result,
    extractor::Component,
    WebConfigurator, WebPlugin,
};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(PgPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[get("/postgres")]
async fn hello_postgres(Component(pg): Component<Postgres>) -> Result<impl IntoResponse> {
    let rows = pg
        .query("select version() as version", &[])
        .await
        .context("query postgresql failed")?;

    let version: String = rows[0].get("version");

    Ok(Json(version))
}
