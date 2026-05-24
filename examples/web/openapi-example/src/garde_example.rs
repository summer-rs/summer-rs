use serde::Deserialize;
use summer_web::axum::Json;
use summer_web::validation::garde::Garde;
use summer_web::{post, post_api};

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
pub struct GardeCreateUserRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(email)]
    pub email: String,
    #[garde(range(min = 0, max = 150))]
    pub age: Option<i32>,
}

#[post_api("/garde/users")]
async fn garde_create_user(
    Garde(Json(body)): Garde<Json<GardeCreateUserRequest>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "age": body.age,
    }))
}

// Register concrete garde context components in the Summer container.

#[derive(Clone, Debug)]
pub struct UserValidationRules {
    pub min_name: usize,
    pub max_name: usize,
}

#[derive(Clone, Debug)]
pub struct PasswordValidationRules {
    pub min_entropy: usize,
}

#[summer::component]
fn create_user_validation_rules() -> UserValidationRules {
    UserValidationRules {
        min_name: 2,
        max_name: 50,
    }
}

#[summer::component]
fn create_password_validation_rules() -> PasswordValidationRules {
    PasswordValidationRules { min_entropy: 4 }
}

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
#[garde(context(UserValidationRules as ctx))]
pub struct GardeContextCreateUserRequest {
    #[garde(length(min = ctx.min_name, max = ctx.max_name))]
    pub name: String,
    #[garde(email)]
    pub email: String,
}

fn validate_password_strength(
    value: &str,
    ctx: &PasswordValidationRules,
) -> Result<(), garde::Error> {
    let score = value.chars().collect::<std::collections::HashSet<_>>().len();
    if score < ctx.min_entropy {
        return Err(garde::Error::new("password is not strong enough"));
    }
    Ok(())
}

#[derive(Debug, Deserialize, summer_web::GardeSchema, garde::Validate)]
#[garde(context(PasswordValidationRules))]
pub struct GardeContextPasswordRequest {
    #[garde(custom(validate_password_strength))]
    pub password: String,
}

#[post("/garde/context/users")]
async fn garde_context_create_user(
    Garde(Json(body)): Garde<Json<GardeContextCreateUserRequest>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 1,
        "name": body.name,
        "email": body.email,
        "rules": "min_name=2, max_name=50 (from UserValidationRules context)",
    }))
}

/// Garde custom validation with Summer-managed runtime context.
///
/// @tag Garde Context Validation
#[post("/garde/context/password")]
async fn garde_context_password(
    Garde(Json(body)): Garde<Json<GardeContextPasswordRequest>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "accepted": true,
        "password_length": body.password.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_operation_input<T: summer_web::aide::OperationInput>() {}

    #[test]
    fn garde_extractors_support_openapi_input() {
        assert_operation_input::<Garde<Json<GardeCreateUserRequest>>>();
    }
}
