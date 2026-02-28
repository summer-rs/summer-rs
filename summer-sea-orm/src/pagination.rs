use schemars::JsonSchema;
use sea_orm::{
    ConnectionTrait, EntityTrait, FromQueryResult, PaginatorTrait, Select, Selector, SelectorTrait,
};
use serde::{Deserialize, Serialize};
use summer::async_trait;
use thiserror::Error;

/// pagination information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_size")]
    pub size: u64,
}

fn default_page() -> u64 {
    0
}
fn default_size() -> u64 {
    20
}

impl Pagination {
    pub fn empty_page<T>(&self) -> Page<T> {
        Page::new(vec![], self, 0)
    }
}

#[cfg(feature = "with-web")]
mod web {
    use super::Pagination;
    use crate::config::SeaOrmWebConfig;
    use schemars::JsonSchema;
    use serde::Deserialize;
    use summer_web::axum::extract::rejection::QueryRejection;
    use summer_web::axum::extract::{FromRequestParts, Query};
    use summer_web::axum::http::request::Parts;
    use summer_web::axum::response::IntoResponse;
    use summer_web::extractor::RequestPartsExt;
    use std::result::Result as StdResult;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum SeaOrmWebErr {
        #[error(transparent)]
        QueryRejection(#[from] QueryRejection),

        #[error(transparent)]
        WebError(#[from] summer_web::error::WebError),
    }

    impl IntoResponse for SeaOrmWebErr {
        fn into_response(self) -> summer_web::axum::response::Response {
            match self {
                Self::QueryRejection(e) => e.into_response(),
                Self::WebError(e) => e.into_response(),
            }
        }
    }

    #[derive(Debug, Clone, Deserialize, JsonSchema)]
    struct OptionalPagination {
        page: Option<u64>,
        size: Option<u64>,
    }

    impl<S> FromRequestParts<S> for Pagination
    where
        S: Sync,
    {
        type Rejection = SeaOrmWebErr;

        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> StdResult<Self, Self::Rejection> {
            let Query(pagination) = Query::<OptionalPagination>::try_from_uri(&parts.uri)?;

            let config = parts.get_config::<SeaOrmWebConfig>()?;

            let size = match pagination.size {
                Some(size) => {
                    if size > config.max_page_size {
                        config.max_page_size
                    } else {
                        size
                    }
                }
                None => config.default_page_size,
            };

            let page = if config.one_indexed {
                pagination
                    .page
                    .map(|page| if page == 0 { 0 } else { page - 1 })
                    .unwrap_or(0)
            } else {
                pagination.page.unwrap_or(0)
            };

            Ok(Pagination { page, size })
        }
    }

    #[cfg(feature = "with-web-openapi")]
    impl summer_web::aide::OperationInput for Pagination {
        fn operation_input(
            ctx: &mut summer_web::aide::generate::GenContext,
            operation: &mut summer_web::aide::openapi::Operation,
        ) {
            <Query<OptionalPagination> as summer_web::aide::OperationInput>::operation_input(
                ctx, operation,
            );
        }

        fn inferred_early_responses(
            ctx: &mut summer_web::aide::generate::GenContext,
            operation: &mut summer_web::aide::openapi::Operation,
        ) -> Vec<(
            Option<summer_web::aide::openapi::StatusCode>,
            summer_web::aide::openapi::Response,
        )> {
            <Query<OptionalPagination> as summer_web::aide::OperationInput>::inferred_early_responses(ctx, operation)
        }
    }
}

/// A page is a sublist of a list of objects.
/// It allows gain information about the position of it in the containing entire list.

#[derive(Debug, Serialize, JsonSchema)]
pub struct Page<T> {
    pub content: Vec<T>,
    pub size: u64,
    pub page: u64,
    /// the total amount of elements.
    pub total_elements: u64,
    /// the number of total pages.
    pub total_pages: u64,
}

impl<T> Page<T> {
    pub fn new(content: Vec<T>, pagination: &Pagination, total: u64) -> Self {
        Self {
            content,
            size: pagination.size,
            page: pagination.page,
            total_elements: total,
            total_pages: Self::total_pages(total, pagination.size),
        }
    }

    /// Compute the number of pages for the current page
    fn total_pages(total: u64, size: u64) -> u64 {
        if size == 0 {
            return 0;
        }
        (total / size) + u64::from(!total.is_multiple_of(size))
    }

    /// iterator for content
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.content.iter()
    }

    /// Returns a new Page with the content of the current one mapped by the given Function
    pub fn map<F, R>(self, func: F) -> Page<R>
    where
        F: FnMut(T) -> R,
    {
        let Page {
            content,
            size,
            page,
            total_elements,
            total_pages,
        } = self;
        let content = content.into_iter().map(func).collect();
        Page {
            content,
            size,
            page,
            total_elements,
            total_pages,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    #[inline]
    pub fn is_first(&self) -> bool {
        self.page == 0
    }

    #[inline]
    pub fn is_last(&self) -> bool {
        self.page + 1 >= self.total_pages
    }
}

#[derive(Debug, Error)]
pub enum OrmError {
    #[error(transparent)]
    DbErr(#[from] sea_orm::DbErr),
}

pub type PageResult<T> = std::result::Result<Page<T>, OrmError>;

#[async_trait]
/// A Trait for any type that can paginate results
pub trait PaginationExt<'db, C, M>
where
    C: ConnectionTrait,
{
    /// pagination
    async fn page(self, db: &'db C, pagination: &Pagination) -> PageResult<M>;
}

#[async_trait]
impl<'db, C, M, E> PaginationExt<'db, C, M> for Select<E>
where
    C: ConnectionTrait,
    E: EntityTrait<Model = M>,
    M: FromQueryResult + Sized + Send + Sync + 'db,
{
    async fn page(self, db: &'db C, pagination: &Pagination) -> PageResult<M> {
        let paginator = self.paginate(db, pagination.size);
        let total = paginator.num_items().await?;
        let content = paginator.fetch_page(pagination.page).await?;
        Ok(Page::new(content, pagination, total))
    }
}

#[async_trait]
impl<'db, C, S> PaginationExt<'db, C, S::Item> for Selector<S>
where
    C: ConnectionTrait,
    S: SelectorTrait + Send + Sync + 'db,
{
    async fn page(self, db: &'db C, pagination: &Pagination) -> PageResult<S::Item> {
        let paginator = self.paginate(db, pagination.size);
        let total = paginator.num_items().await?;
        let content = paginator.fetch_page(pagination.page).await?;
        Ok(Page::new(content, pagination, total))
    }
}
