//! Garde wrappers returning RFC 9457 ProblemDetails on failure.
//!
//! Provides a generic `Garde<E>` wrapper for the built-in `Json<T>`,
//! `Query<T>`, `Path<T>`, and `Form<T>` extractors.

use crate::problem_details::{ProblemDetails, ViolationLocation};
use crate::validation::{
    form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
    query_rejection_to_problem,
};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use garde::Validate;
use std::any::{Any, TypeId};
use summer::plugin::component::ComponentRef;
use summer::plugin::ComponentRegistry;

/// Generic garde wrapper for extractors whose inner type implements `garde::Validate`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Garde<E>(pub E);

impl<E> std::ops::Deref for Garde<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> std::ops::DerefMut for Garde<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

enum GardeInternalError {
    MissingContext(&'static str),
    ValidationReport(garde::Report, ViolationLocation),
}

impl From<GardeInternalError> for ProblemDetails {
    fn from(error: GardeInternalError) -> Self {
        match error {
            GardeInternalError::MissingContext(context_type) => ProblemDetails::new(500)
                .with_detail(format!(
                    "Server validation configuration for '{}' is unavailable.",
                    context_type
                )),
            GardeInternalError::ValidationReport(report, location) => {
                ProblemDetails::from_garde_report(&report, location)
            }
        }
    }
}

fn fetch_context<T: Validate>(
    parts: &Parts,
) -> Result<Option<ComponentRef<T::Context>>, GardeInternalError>
where
    T::Context: Clone + Send + Sync + 'static,
{
    if TypeId::of::<T::Context>() == TypeId::of::<()>() {
        Ok(None)
    } else {
        use crate::extractor::RequestPartsExt;
        let context = parts
            .get_app_state()
            .app
            .try_get_component_ref::<T::Context>()
            .map_err(|_| GardeInternalError::MissingContext(std::any::type_name::<T::Context>()))?;
        Ok(Some(context))
    }
}

fn validate_without_context<T: Validate>(data: &T) -> Result<(), garde::Report>
where
    T::Context: 'static,
{
    let unit = ();
    let ctx = (&unit as &dyn Any)
        .downcast_ref::<T::Context>()
        .expect("TypeId check guarantees T::Context is ()");
    data.validate_with(ctx)
}

fn validate_data<T: Validate>(
    data: &T,
    context: Option<&T::Context>,
    location: ViolationLocation,
) -> Result<(), GardeInternalError>
where
    T::Context: Send + Sync + 'static,
{
    let result = if TypeId::of::<T::Context>() == TypeId::of::<()>() {
        validate_without_context(data)
    } else {
        let context =
            context.ok_or(GardeInternalError::MissingContext(std::any::type_name::<T::Context>()))?;
        data.validate_with(context)
    };

    result.map_err(|report| GardeInternalError::ValidationReport(report, location))
}

impl<State, T> FromRequest<State> for Garde<axum::Json<T>>
where
    State: Send + Sync,
    T: Validate,
    T::Context: Clone + Send + Sync + 'static,
    axum::Json<T>: FromRequest<State, Rejection = axum::extract::rejection::JsonRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let context = fetch_context::<T>(&parts)?;
        let req = Request::from_parts(parts, body);

        let inner = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;
        validate_data(&inner.0, context.as_deref(), ViolationLocation::Body)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequest<State> for Garde<axum::extract::Form<T>>
where
    State: Send + Sync,
    T: Validate,
    T::Context: Clone + Send + Sync + 'static,
    axum::extract::Form<T>:
        FromRequest<State, Rejection = axum::extract::rejection::FormRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request(req: Request, state: &State) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let context = fetch_context::<T>(&parts)?;
        let req = Request::from_parts(parts, body);

        let inner = axum::extract::Form::<T>::from_request(req, state)
            .await
            .map_err(form_rejection_to_problem)?;
        validate_data(&inner.0, context.as_deref(), ViolationLocation::Form)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequestParts<State> for Garde<axum::extract::Query<T>>
where
    State: Send + Sync,
    T: Validate,
    T::Context: Clone + Send + Sync + 'static,
    axum::extract::Query<T>:
        FromRequestParts<State, Rejection = axum::extract::rejection::QueryRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let context = fetch_context::<T>(parts)?;
        let inner = axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(query_rejection_to_problem)?;
        validate_data(&inner.0, context.as_deref(), ViolationLocation::Query)?;
        Ok(Self(inner))
    }
}

impl<State, T> FromRequestParts<State> for Garde<axum::extract::Path<T>>
where
    State: Send + Sync,
    T: Validate,
    T::Context: Clone + Send + Sync + 'static,
    axum::extract::Path<T>:
        FromRequestParts<State, Rejection = axum::extract::rejection::PathRejection>,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, state: &State) -> Result<Self, Self::Rejection> {
        let context = fetch_context::<T>(parts)?;
        let inner = axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(path_rejection_to_problem)?;
        validate_data(&inner.0, context.as_deref(), ViolationLocation::Path)?;
        Ok(Self(inner))
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::Json<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::Json<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::extract::Query<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Query<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::extract::Path<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Path<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(feature = "openapi")]
impl<T: schemars::JsonSchema> aide::OperationInput for Garde<axum::extract::Form<T>> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        <axum::extract::Form<T> as aide::OperationInput>::operation_input(ctx, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::GardeInternalError;
    use crate::problem_details::ViolationLocation;

    #[test]
    fn internal_validation_report_converts_to_problem_details() {
        let mut report = garde::Report::new();
        report.append(garde::Path::new("name"), garde::Error::new("too short"));

        let problem = crate::problem_details::ProblemDetails::from(
            GardeInternalError::ValidationReport(report, ViolationLocation::Body),
        );

        assert_eq!(problem.status, 400);
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "name");
        assert_eq!(problem.violations[0].message, "too short");
    }
}

/// When `axum-valid` feature is also enabled, provide `From<GardeRejection<*>>` impls
/// so users can use `axum_valid::Garde<T>` directly and still get ProblemDetails conversion.
#[cfg(feature = "axum-valid")]
mod axum_valid_compat {
    use super::*;
    use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};
    use axum_valid::GardeRejection;
    use crate::validation::{
        form_rejection_to_problem, json_rejection_to_problem, path_rejection_to_problem,
        query_rejection_to_problem,
    };

    impl From<GardeRejection<JsonRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<JsonRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Body)
                }
                GardeRejection::Inner(inner) => json_rejection_to_problem(inner),
            }
        }
    }

    impl From<GardeRejection<QueryRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<QueryRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Query)
                }
                GardeRejection::Inner(inner) => query_rejection_to_problem(inner),
            }
        }
    }

    impl From<GardeRejection<PathRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<PathRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Path)
                }
                GardeRejection::Inner(inner) => path_rejection_to_problem(inner),
            }
        }
    }

    impl From<GardeRejection<FormRejection>> for ProblemDetails {
        fn from(rejection: GardeRejection<FormRejection>) -> Self {
            match rejection {
                GardeRejection::Valid(report) => {
                    ProblemDetails::from_garde_report(&report, ViolationLocation::Form)
                }
                GardeRejection::Inner(inner) => form_rejection_to_problem(inner),
            }
        }
    }
}
