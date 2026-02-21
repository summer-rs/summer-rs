mod entities;

use anyhow::Context;
use entities::{
    prelude::{TodoItem, TodoList},
    todo_item, todo_list,
};
use sea_orm::{sea_query::IntoCondition, ColumnTrait, Condition, EntityTrait, QueryFilter};
use serde::Deserialize;
use spring::{auto_config, App};
use spring_sea_orm::{
    pagination::{Pagination, PaginationExt},
    DbConn, SeaOrmPlugin,
};
use spring_web::get;
use spring_web::{
    axum::response::{IntoResponse, Json},
    error::Result,
    extractor::{Component, Path, Query},
    WebConfigurator, WebPlugin,
};

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(SeaOrmPlugin)
        .add_plugin(WebPlugin)
        .run()
        .await
}

#[derive(Deserialize)]
struct TodoListQuery {
    title: Option<String>,
}

impl From<TodoListQuery> for Condition {
    fn from(query: TodoListQuery) -> Self {
        match query.title {
            Some(title) => todo_list::Column::Title.starts_with(title).into_condition(),
            None => Condition::all(),
        }
    }
}

#[get("/")]
async fn get_todo_list(
    Component(db): Component<DbConn>,
    Query(query): Query<TodoListQuery>,
    pagination: Pagination,
) -> Result<impl IntoResponse> {
    let rows = TodoList::find()
        .filter(query)
        .page(&db, &pagination)
        .await
        .context("query todo list failed")?;
    Ok(Json(rows))
}

#[get("/{id}")]
async fn get_todo_list_items(
    Component(db): Component<DbConn>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse> {
    let rows = TodoItem::find()
        .filter(todo_item::Column::ListId.eq(id))
        .all(&db)
        .await
        .context("query todo list failed")?;
    Ok(Json(rows))
}
