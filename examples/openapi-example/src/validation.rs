use summer_web::axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use summer_web::axum::http::request::Parts;
use summer_web::axum::Json;
use summer_web::extractor::{FromRequest, FromRequestParts, Path, Query};
use summer_web::problem_details::{ProblemDetails, Violation, ViolationLocation};
use validator::{Validate, ValidationErrors, ValidationErrorsKind};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Custom Extractors
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Two-phase validation:
//   1. Deserialization (serde)  → catches missing fields, wrong types
//   2. Validation   (validator) → catches business rules (email, range, length…)
//
// Both phases produce ProblemDetails with field-level violations.

/// JSON body extractor with validation.
///
/// Phase 1: deserializes the body via `Json<T>`.
/// Phase 2: calls `T::validate()` to run `#[validate(...)]` rules.
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request(
        req: summer_web::axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;

        data.validate()
            .map_err(|e| validation_errors_to_problem(&e, ViolationLocation::Body))?;

        Ok(ValidatedJson(data))
    }
}

impl<T: schemars::JsonSchema> summer_web::aide::OperationInput for ValidatedJson<T> {
    fn operation_input(
        ctx: &mut summer_web::aide::generate::GenContext,
        operation: &mut summer_web::aide::openapi::Operation,
    ) {
        <Json<T> as summer_web::aide::OperationInput>::operation_input(ctx, operation);
    }
}

/// Query parameter extractor with validation.
pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Query(data) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(query_rejection_to_problem)?;

        data.validate()
            .map_err(|e| validation_errors_to_problem(&e, ViolationLocation::Query))?;

        Ok(ValidatedQuery(data))
    }
}

impl<T: schemars::JsonSchema> summer_web::aide::OperationInput for ValidatedQuery<T> {
    fn operation_input(
        ctx: &mut summer_web::aide::generate::GenContext,
        operation: &mut summer_web::aide::openapi::Operation,
    ) {
        <Query<T> as summer_web::aide::OperationInput>::operation_input(ctx, operation);
    }
}

/// Path parameter extractor with validation.
pub struct ValidatedPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedPath<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(data) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(path_rejection_to_problem)?;

        data.validate()
            .map_err(|e| validation_errors_to_problem(&e, ViolationLocation::Path))?;

        Ok(ValidatedPath(data))
    }
}

impl<T: schemars::JsonSchema> summer_web::aide::OperationInput for ValidatedPath<T> {
    fn operation_input(
        ctx: &mut summer_web::aide::generate::GenContext,
        operation: &mut summer_web::aide::openapi::Operation,
    ) {
        <Path<T> as summer_web::aide::OperationInput>::operation_input(ctx, operation);
    }
}

/// JSON body extractor WITHOUT validation.
///
/// Only performs Phase 1 (serde deserialization).
/// `T` does NOT need to implement `Validate`.
/// Deserialization failures are still returned as ProblemDetails.
pub struct JsonWithProblem<T>(pub T);

impl<T, S> FromRequest<S> for JsonWithProblem<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ProblemDetails;

    async fn from_request(
        req: summer_web::axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(data) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection_to_problem)?;

        Ok(JsonWithProblem(data))
    }
}

impl<T: schemars::JsonSchema> summer_web::aide::OperationInput for JsonWithProblem<T> {
    fn operation_input(
        ctx: &mut summer_web::aide::generate::GenContext,
        operation: &mut summer_web::aide::openapi::Operation,
    ) {
        <Json<T> as summer_web::aide::OperationInput>::operation_input(ctx, operation);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ValidationErrors → ProblemDetails
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn validation_errors_to_problem(
    errors: &ValidationErrors,
    location: ViolationLocation,
) -> ProblemDetails {
    let mut violations = Vec::new();
    collect_violations(errors, &location, "", &mut violations);
    ProblemDetails::validation_error_with_violations(violations)
}

fn collect_violations(
    errors: &ValidationErrors,
    location: &ViolationLocation,
    prefix: &str,
    violations: &mut Vec<Violation>,
) {
    for (field, kind) in errors.errors() {
        let full_field = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };

        match kind {
            ValidationErrorsKind::Field(errs) => {
                for err in errs {
                    let message = format_validation_message(err);
                    violations.push(Violation::new(&full_field, location.clone(), message));
                }
            }
            ValidationErrorsKind::Struct(nested) => {
                collect_violations(nested, location, &full_field, violations);
            }
            ValidationErrorsKind::List(map) => {
                for (index, nested) in map {
                    let indexed = format!("{full_field}[{index}]");
                    collect_violations(nested, location, &indexed, violations);
                }
            }
        }
    }
}

fn format_validation_message(err: &validator::ValidationError) -> String {
    // Prefer custom message if provided
    if let Some(msg) = &err.message {
        return msg.to_string();
    }

    // Generate a friendly description based on the error code
    match err.code.as_ref() {
        "required" => "this field is required".to_string(),
        "email" => "must be a valid email address".to_string(),
        "url" => "must be a valid URL".to_string(),
        "length" => {
            let min = err.params.get("min").and_then(|v| v.as_u64());
            let max = err.params.get("max").and_then(|v| v.as_u64());
            match (min, max) {
                (Some(min), Some(max)) => format!("length must be between {min} and {max}"),
                (Some(min), None) => format!("length must be at least {min}"),
                (None, Some(max)) => format!("length must be at most {max}"),
                _ => "invalid length".to_string(),
            }
        }
        "range" => {
            let min = err.params.get("min").and_then(|v| v.as_f64());
            let max = err.params.get("max").and_then(|v| v.as_f64());
            match (min, max) {
                (Some(min), Some(max)) => format!("must be between {min} and {max}"),
                (Some(min), None) => format!("must be at least {min}"),
                (None, Some(max)) => format!("must be at most {max}"),
                _ => "value out of range".to_string(),
            }
        }
        "regex" => "format is invalid".to_string(),
        "contains" => {
            if let Some(pattern) = err.params.get("pattern").and_then(|v| v.as_str()) {
                format!("must contain '{pattern}'")
            } else {
                "missing required content".to_string()
            }
        }
        "must_match" => {
            if let Some(other) = err.params.get("other").and_then(|v| v.as_str()) {
                format!("must match field '{other}'")
            } else {
                "fields do not match".to_string()
            }
        }
        "custom" => "validation failed".to_string(),
        code => format!("validation failed: {code}"),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Rejection → ProblemDetails (phase 1: deserialization errors)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn json_rejection_to_problem(rejection: JsonRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    match rejection {
        JsonRejection::JsonDataError(_) => {
            let stripped = msg
                .strip_prefix(
                    "Failed to deserialize the JSON body into the target type: ",
                )
                .unwrap_or(&msg);

            let (field, detail) = split_field_message(stripped);

            if !field.is_empty() && field != "." {
                let friendly = humanize_serde_message(detail);
                ProblemDetails::validation_error_with_violations(vec![
                    Violation::body(field, friendly),
                ])
            } else if let Some(name) = extract_backtick_value(stripped, "missing field `") {
                ProblemDetails::validation_error_with_violations(vec![
                    Violation::body(name, "this field is required"),
                ])
            } else {
                ProblemDetails::validation_error(humanize_serde_message(stripped))
            }
        }
        JsonRejection::JsonSyntaxError(_) => {
            ProblemDetails::validation_error("request body is not valid JSON")
        }
        JsonRejection::MissingJsonContentType(_) => {
            ProblemDetails::new("about:blank", "Unsupported Media Type", 415)
                .with_detail("expected Content-Type: application/json")
        }
        _ => ProblemDetails::validation_error("failed to read request body"),
    }
}

fn query_rejection_to_problem(rejection: QueryRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    let stripped = msg
        .strip_prefix("Failed to deserialize query string: ")
        .unwrap_or(&msg);

    if let Some(name) = extract_backtick_value(stripped, "missing field `") {
        ProblemDetails::validation_error_with_violations(vec![
            Violation::query(name, "this query parameter is required"),
        ])
    } else {
        // Format: "page: invalid digit found in string"
        let (field, detail) = split_field_message(stripped);
        if !field.is_empty() {
            ProblemDetails::validation_error_with_violations(vec![
                Violation::query(field, humanize_serde_message(detail)),
            ])
        } else {
            ProblemDetails::validation_error_with_violations(vec![
                Violation::query("query", humanize_serde_message(stripped)),
            ])
        }
    }
}

fn path_rejection_to_problem(rejection: PathRejection) -> ProblemDetails {
    let msg = rejection.body_text();

    // axum 0.8 format: "Invalid URL: Cannot parse `id` with value `abc` to a `u32`"
    if let Some(name) = extract_backtick_value(&msg, "Cannot parse `") {
        let expected = extract_backtick_value(&msg, "to a `").unwrap_or("valid value");
        ProblemDetails::validation_error_with_violations(vec![
            Violation::path(name, format!("must be a valid {expected}")),
        ])
    } else {
        ProblemDetails::validation_error_with_violations(vec![
            Violation::path("path", "invalid path parameter"),
        ])
    }
}

// ── Parsing helpers ────────────────────────────────────────────────

fn split_field_message(s: &str) -> (&str, &str) {
    match s.find(": ") {
        Some(pos) => (&s[..pos], &s[pos + 2..]),
        None => ("", s),
    }
}

/// Extract the value between backticks after a given prefix.
/// e.g. `extract_backtick_value(msg, "missing field \`")` → Some("name")
fn extract_backtick_value<'a>(msg: &'a str, prefix: &str) -> Option<&'a str> {
    let start = msg.find(prefix)?;
    let rest = &msg[start + prefix.len()..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

/// Turn serde's raw error into a human-friendly sentence.
fn humanize_serde_message(raw: &str) -> String {
    // Strip " at line X column Y" suffix
    let clean = if let Some(pos) = raw.find(" at line ") {
        &raw[..pos]
    } else {
        raw
    };

    if clean.starts_with("missing field") {
        if let Some(name) = extract_backtick_value(clean, "missing field `") {
            return format!("field '{name}' is required");
        }
    }

    if clean.starts_with("invalid type:") {
        let detail = clean.strip_prefix("invalid type: ").unwrap_or(clean);
        return format!("invalid type: {detail}");
    }

    if clean.starts_with("unknown field") {
        return clean.to_string();
    }

    clean.to_string()
}
