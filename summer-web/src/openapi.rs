use aide::{Error, generate::GenContext, openapi::{Operation, ReferenceOr, Response, StatusCode}};

// Re-export Problem Details OpenAPI utilities
#[cfg(feature = "openapi")]
pub use crate::problem_details::{
    ProblemDetailsVariantInfo,
    problem_details_schema,
    register_error_response_by_variant,
};

pub fn set_inferred_response(
    ctx: &mut GenContext,
    operation: &mut Operation,
    status: Option<StatusCode>,
    res: Response,
) {
    if operation.responses.is_none() {
        operation.responses = Some(Default::default());
    }

    let responses = operation.responses.as_mut().unwrap();

    match status {
        Some(status_code) => {
            let Some(existing) = responses.responses.get_mut(&status_code) else {
                responses
                    .responses
                    .insert(status_code, ReferenceOr::Item(res));
                return;
            };

            let ReferenceOr::Item(existing_response) = existing else {
                return;
            };

            if existing_response.description != res.description {
                existing_response.description = format!("- {}\n- {}", existing_response.description, res.description);
            }
        }
        None => {
            if responses.default.is_some() {
                ctx.error(Error::InferredDefaultResponseConflict);
            } else {
                responses.default = Some(ReferenceOr::Item(res));
            }
        }
    }
}
