//! Validation extractors that return RFC 9457 ProblemDetails on failure.
//!
//! This module provides framework-level wrappers around validation extractors so
//! deserialization and validation failures become structured
//! [`ProblemDetails`](crate::problem_details::ProblemDetails) responses with
//! field-level [`Violation`](crate::problem_details::Violation) information.

use crate::problem_details::{ProblemDetails, Violation};
use axum::extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection};

#[cfg(feature = "garde")]
pub mod garde;
#[cfg(feature = "validator")]
pub mod validator;

fn json_rejection_to_problem(rejection: JsonRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    match rejection {
        JsonRejection::JsonDataError(_) => {
            let stripped = msg
                .strip_prefix("Failed to deserialize the JSON body into the target type: ")
                .unwrap_or(&msg);

            let (field, detail) = split_field_message(stripped);

            if !field.is_empty() && field != "." {
                ProblemDetails::validation_error(vec![Violation::body(
                    field,
                    humanize_serde_message(detail),
                )])
            } else if let Some(name) = extract_backtick_value(stripped, "missing field `") {
                ProblemDetails::validation_error(vec![Violation::body(
                    name,
                    "this field is required",
                )])
            } else {
                ProblemDetails::validation_error_simple(humanize_serde_message(stripped))
            }
        }
        JsonRejection::JsonSyntaxError(_) => {
            ProblemDetails::validation_error_simple("request body is not valid JSON")
        }
        JsonRejection::MissingJsonContentType(_) => {
            ProblemDetails::new(415).with_detail("expected Content-Type: application/json")
        }
        _ => ProblemDetails::validation_error_simple("failed to read request body"),
    }
}

fn query_rejection_to_problem(rejection: QueryRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    let stripped = msg
        .strip_prefix("Failed to deserialize query string: ")
        .unwrap_or(&msg);

    if let Some(name) = extract_backtick_value(stripped, "missing field `") {
        ProblemDetails::validation_error(vec![Violation::query(
            name,
            "this query parameter is required",
        )])
    } else {
        let (field, detail) = split_field_message(stripped);
        if !field.is_empty() {
            ProblemDetails::validation_error(vec![Violation::query(
                field,
                humanize_serde_message(detail),
            )])
        } else {
            ProblemDetails::validation_error(vec![Violation::query(
                "query",
                humanize_serde_message(stripped),
            )])
        }
    }
}

fn path_rejection_to_problem(rejection: PathRejection) -> ProblemDetails {
    let msg = rejection.body_text();

    if let Some(name) = extract_backtick_value(&msg, "Cannot parse `") {
        let expected = extract_backtick_value(&msg, "to a `").unwrap_or("valid value");
        ProblemDetails::validation_error(vec![Violation::path(
            name,
            format!("must be a valid {expected}"),
        )])
    } else {
        ProblemDetails::validation_error(vec![Violation::path("path", "invalid path parameter")])
    }
}

fn form_rejection_to_problem(rejection: FormRejection) -> ProblemDetails {
    let msg = rejection.body_text();
    let stripped = msg
        .strip_prefix("Failed to deserialize form: ")
        .or_else(|| msg.strip_prefix("Failed to deserialize form body: "))
        .unwrap_or(&msg);

    match rejection {
        FormRejection::InvalidFormContentType(_) => ProblemDetails::new(415)
            .with_detail("expected Content-Type: application/x-www-form-urlencoded"),
        FormRejection::FailedToDeserializeForm(_)
        | FormRejection::FailedToDeserializeFormBody(_) => {
            if let Some(name) = extract_backtick_value(stripped, "missing field `") {
                ProblemDetails::validation_error(vec![Violation::form(
                    name,
                    "this form field is required",
                )])
            } else {
                let (field, detail) = split_field_message(stripped);
                if !field.is_empty() {
                    ProblemDetails::validation_error(vec![Violation::form(
                        field,
                        humanize_serde_message(detail),
                    )])
                } else {
                    ProblemDetails::validation_error(vec![Violation::form(
                        "form",
                        humanize_serde_message(stripped),
                    )])
                }
            }
        }
        FormRejection::BytesRejection(_) => {
            ProblemDetails::validation_error_simple("failed to read form body")
        }
        _ => ProblemDetails::validation_error_simple("failed to read form body"),
    }
}

fn split_field_message(s: &str) -> (&str, &str) {
    match s.find(": ") {
        Some(pos) => (&s[..pos], &s[pos + 2..]),
        None => ("", s),
    }
}

fn extract_backtick_value<'a>(msg: &'a str, prefix: &str) -> Option<&'a str> {
    let start = msg.find(prefix)?;
    let rest = &msg[start + prefix.len()..];
    let end = rest.find('`')?;
    Some(&rest[..end])
}

fn humanize_serde_message(raw: &str) -> String {
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

    clean.to_string()
}
