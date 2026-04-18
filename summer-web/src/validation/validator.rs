//! Validator wrappers returning RFC 9457 ProblemDetails on failure.
//!
//! Provides two generic wrappers:
//! 1. `Validator<E>` for `validator::Validate`
//! 2. `ValidatorEx<E>` for `validator::ValidateArgs`
//!
//! The built-in supported extractors are `Json<T>`, `Query<T>`, `Path<T>`, and
//! `Form<T>`.

use crate::problem_details::{ProblemDetails, ViolationLocation};
use crate::validation::{
    form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
    query_rejection_to_problem,
};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use summer::plugin::component::ComponentRef;
use summer::plugin::ComponentRegistry;
use validator::{Validate, ValidateArgs};

/// Runtime metadata for validator types that use `ValidateArgs`.
pub trait ValidatorContextType {
    type Context: Send + Sync + 'static;
}

enum ValidatorInternalError {
    MissingContext(&'static str),
    ValidationErrors(validator::ValidationErrors, ViolationLocation),
}

impl From<ValidatorInternalError> for ProblemDetails {
    fn from(error: ValidatorInternalError) -> Self {
        match error {
            ValidatorInternalError::MissingContext(args_type) => ProblemDetails::new(500)
                .with_detail(format!(
                    "Server validation configuration for '{}' is unavailable.",
                    args_type
                )),
            ValidatorInternalError::ValidationErrors(errors, location) => {
                ProblemDetails::from_validation_errors(&errors, location)
            }
        }
    }
}

fn validate_data<T: Validate>(
    data: &T,
    location: ViolationLocation,
) -> Result<(), ValidatorInternalError> {
    data.validate()
        .map_err(|errors| ValidatorInternalError::ValidationErrors(errors, location))
}

fn fetch_context<C>(parts: &Parts) -> Result<ComponentRef<C>, ValidatorInternalError>
where
    C: Clone + Send + Sync + 'static,
{
    use crate::extractor::RequestPartsExt;

    parts
        .get_app_state()
        .app
        .try_get_component_ref::<C>()
        .map_err(|_| ValidatorInternalError::MissingContext(std::any::type_name::<C>()))
}

fn validate_data_with_args<T>(
    data: &T,
    context: &T::Context,
    location: ViolationLocation,
) -> Result<(), ValidatorInternalError>
where
    T: ValidatorContextType,
    for<'v> T: ValidateArgs<'v, Args = &'v T::Context>,
{
    data.validate_with_args(context)
        .map_err(|errors| ValidatorInternalError::ValidationErrors(errors, location))
}

/// Generic validator wrapper for extractors whose inner type implements `Validate`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Validator<E>(pub E);

impl<E> std::ops::Deref for Validator<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> std::ops::DerefMut for Validator<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Generic validator wrapper for extractors whose inner type implements `ValidateArgs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatorEx<E>(pub E);

impl<E> std::ops::Deref for ValidatorEx<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> std::ops::DerefMut for ValidatorEx<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<State, T> FromRequest<State> for Validator<axum::Json<T>>
where
    State: Send + Sync,
    T: Validate,
    axum::Json<T>: FromRequest<State, Rejection = axum::extract::rejection::JsonRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let inner = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;
        validate_data(&inner.0, ViolationLocation::Body)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequest<State> for Validator<axum::extract::Form<T>>
where
    State: Send + Sync,
    T: Validate,
    axum::extract::Form<T>:
        FromRequest<State, Rejection = axum::extract::rejection::FormRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let inner = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(form_rejection_to_problem)?;
        validate_data(&inner.0, ViolationLocation::Form)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequestParts<State> for Validator<axum::extract::Query<T>>
where
    State: Send + Sync,
    T: Validate,
    axum::extract::Query<T>:
        FromRequestParts<State, Rejection = axum::extract::rejection::QueryRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let inner = axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(query_rejection_to_problem)?;
        validate_data(&inner.0, ViolationLocation::Query)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequestParts<State> for Validator<axum::extract::Path<T>>
where
    State: Send + Sync,
    T: Validate,
    axum::extract::Path<T>:
        FromRequestParts<State, Rejection = axum::extract::rejection::PathRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let inner = axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(path_rejection_to_problem)?;
        validate_data(&inner.0, ViolationLocation::Path)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequest<State> for ValidatorEx<axum::Json<T>>
where
    State: Send + Sync,
    T: ValidatorContextType,
    T::Context: Clone + Send + Sync + 'static,
    for<'v> T: ValidateArgs<'v, Args = &'v T::Context>,
    axum::Json<T>: FromRequest<State, Rejection = axum::extract::rejection::JsonRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let context = fetch_context::<T::Context>(&parts)?;
        let req = Request::from_parts(parts, body);

        let inner = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;
        validate_data_with_args(&inner.0, &context, ViolationLocation::Body)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequest<State> for ValidatorEx<axum::extract::Form<T>>
where
    State: Send + Sync,
    T: ValidatorContextType,
    T::Context: Clone + Send + Sync + 'static,
    for<'v> T: ValidateArgs<'v, Args = &'v T::Context>,
    axum::extract::Form<T>:
        FromRequest<State, Rejection = axum::extract::rejection::FormRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let context = fetch_context::<T::Context>(&parts)?;
        let req = Request::from_parts(parts, body);

        let inner = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(form_rejection_to_problem)?;
        validate_data_with_args(&inner.0, &context, ViolationLocation::Form)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequestParts<State> for ValidatorEx<axum::extract::Query<T>>
where
    State: Send + Sync,
    T: ValidatorContextType,
    T::Context: Clone + Send + Sync + 'static,
    for<'v> T: ValidateArgs<'v, Args = &'v T::Context>,
    axum::extract::Query<T>:
        FromRequestParts<State, Rejection = axum::extract::rejection::QueryRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let context = fetch_context::<T::Context>(parts)?;
        let inner = axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(query_rejection_to_problem)?;
        validate_data_with_args(&inner.0, &context, ViolationLocation::Query)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequestParts<State> for ValidatorEx<axum::extract::Path<T>>
where
    State: Send + Sync,
    T: ValidatorContextType,
    T::Context: Clone + Send + Sync + 'static,
    for<'v> T: ValidateArgs<'v, Args = &'v T::Context>,
    axum::extract::Path<T>:
        FromRequestParts<State, Rejection = axum::extract::rejection::PathRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let context = fetch_context::<T::Context>(parts)?;
        let inner = axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(path_rejection_to_problem)?;
        validate_data_with_args(&inner.0, &context, ViolationLocation::Path)?;
        Ok(Self(inner))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::Json<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::extract::Query<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::extract::Path<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Validator<axum::extract::Form<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::Json<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::extract::Query<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::extract::Path<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for ValidatorEx<axum::extract::Form<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::ValidatorInternalError;
    use crate::problem_details::ViolationLocation;

    #[test]
    fn internal_validation_errors_convert_to_problem_details() {
        let mut errors = validator::ValidationErrors::new();
        errors.add("name", validator::ValidationError::new("required"));

        let problem = crate::problem_details::ProblemDetails::from(
            ValidatorInternalError::ValidationErrors(errors, ViolationLocation::Body),
        );

        assert_eq!(problem.status, 400);
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "name");
        assert_eq!(problem.violations[0].location, ViolationLocation::Body);
    }
}

/// When `axum-valid` feature is also enabled, provide `From<ValidationRejection<*>>`
/// impls so users can use `axum_valid::Valid<T>` directly and still get ProblemDetails.
#[cfg(feature = "axum-valid")]
mod axum_valid_compat {
    use super::*;
    use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};
    use axum_valid::ValidationRejection;
    use crate::validation::{
        form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
        query_rejection_to_problem,
    };
    use validator::ValidationErrors;

    impl From<ValidationRejection<ValidationErrors, JsonRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, JsonRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Body)
                }
                ValidationRejection::Inner(inner) => json_rejection_to_problem(inner),
            }
        }
    }

    impl From<ValidationRejection<ValidationErrors, QueryRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, QueryRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Query)
                }
                ValidationRejection::Inner(inner) => query_rejection_to_problem(inner),
            }
        }
    }

    impl From<ValidationRejection<ValidationErrors, PathRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, PathRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Path)
                }
                ValidationRejection::Inner(inner) => path_rejection_to_problem(inner),
            }
        }
    }

    impl From<ValidationRejection<ValidationErrors, FormRejection>> for ProblemDetails {
        fn from(rejection: ValidationRejection<ValidationErrors, FormRejection>) -> Self {
            match rejection {
                ValidationRejection::Valid(errors) => {
                    ProblemDetails::from_validation_errors(&errors, ViolationLocation::Form)
                }
                ValidationRejection::Inner(inner) => form_rejection_to_problem(inner),
            }
        }
    }
}
