//! Multi-datasource Example
//!
//! This example demonstrates how to configure and use multiple database connections
//! in a summer-rs application. This is useful for:
//! - Read/Write splitting (primary for writes, secondary for reads)
//! - Connecting to multiple different databases
//! - Database sharding scenarios
//!
//! # Usage
//!
//! Set the environment variables before running:
//! ```bash
//! export PRIMARY_DATABASE_URL="postgres://user:pass@localhost:5432/primary_db"
//! export SECONDARY_DATABASE_URL="postgres://user:pass@localhost:5432/secondary_db"
//! cargo run -p multi-datasource-example
//! ```

mod multi_datasource;

use anyhow::Context;
use multi_datasource::{MultiDatasourcePlugin, PrimaryDb, SecondaryDb};
use sea_orm::ConnectionTrait;
use summer::{auto_config, App};
use summer_web::get;
use summer_web::{
    axum::response::IntoResponse, error::Result, extractor::Component, WebConfigurator, WebPlugin,
};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(MultiDatasourcePlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

/// Example handler that reads from the primary database
#[get("/primary")]
async fn query_primary(Component(db): Component<PrimaryDb>) -> Result<impl IntoResponse> {
    // Example: Execute a simple query on the primary database
    let _result = db
        .execute_unprepared("SELECT 'Hello from Primary DB' as message")
        .await
        .context("query primary database failed")?;

    Ok("Hello from Primary DB")
}

/// Example handler that reads from the secondary database
#[get("/secondary")]
async fn query_secondary(Component(db): Component<SecondaryDb>) -> Result<impl IntoResponse> {
    // Example: Execute a simple query on the secondary database
    let _result = db
        .execute_unprepared("SELECT 'Hello from Secondary DB' as message")
        .await
        .context("query secondary database failed")?;

    Ok("Hello from Secondary DB")
}

/// Example handler that uses both databases
/// This demonstrates a common read/write splitting pattern:
/// - Write operations go to primary
/// - Read operations go to secondary
#[get("/both")]
async fn query_both(
    Component(primary): Component<PrimaryDb>,
    Component(secondary): Component<SecondaryDb>,
) -> Result<impl IntoResponse> {
    // Query primary database
    let _primary_result = primary
        .execute_unprepared("SELECT current_database() as db_name")
        .await
        .context("query primary database failed")?;

    // Query secondary database
    let _secondary_result = secondary
        .execute_unprepared("SELECT current_database() as db_name")
        .await
        .context("query secondary database failed")?;

    Ok("Primary and Secondary databases are connected")
}
