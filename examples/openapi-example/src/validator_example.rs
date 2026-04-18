use serde::Deserialize;
use summer_web::axum::Json;
use summer_web::validation::validator::{Validator, ValidatorEx};
use summer_web::post_api;
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate)]
pub struct CreateUserRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[validate(email(message = "must be a valid email address"))]
    pub email: String,
    #[validate(range(min = 0, max = 150, message = "age must be between 0 and 150"))]
    pub age: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct PageRules {
    pub max_page_size: usize,
}

fn validate_page_size(value: usize, ctx: &PageRules) -> Result<(), ValidationError> {
    if value > ctx.max_page_size {
        return Err(ValidationError::new("page_size_too_large"));
    }
    Ok(())
}

#[derive(Debug, Deserialize, summer_web::ValidatorSchema, Validate, summer_web::ValidatorContext)]
#[validate(context = PageRules)]
pub struct ValidatorContextQuery {
    #[validate(custom(function = "validate_page_size", use_context))]
    pub page_size: usize,
}

#[summer::component]
fn create_validator_rules() -> PageRules {
    PageRules { max_page_size: 100 }
}

#[post_api("/validation/users")]
async fn create_user(
    Validator(Json(body)): Validator<Json<CreateUserRequest>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "age": body.age,
    }))
}

#[post_api("/validation/users/context")]
async fn validator_context_users(
    ValidatorEx(Json(payload)): ValidatorEx<Json<ValidatorContextQuery>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "page_size": payload.page_size,
        "rules": "max_page_size=100 (from PageRules component)",
    }))
}
