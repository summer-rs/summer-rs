#[cfg(feature = "validator")]
mod validator_args {
    use summer::plugin::MutableComponentRegistry;
    use summer_web::axum::body::Body;
    use summer_web::axum::http::{Request, StatusCode};
    use summer_web::axum::Extension;
    use summer_web::axum::Json;
    use summer_web::problem_details::{ProblemDetails, ViolationLocation};
    use summer_web::{AppState, Router};
    use tower::ServiceExt;
    use validator::{Validate, ValidationError};

    use serde::Deserialize;

    #[derive(Clone, Debug)]
    struct PageRules {
        max_page_size: usize,
    }

    fn validate_page_size(value: usize, ctx: &PageRules) -> Result<(), ValidationError> {
        if value > ctx.max_page_size {
            return Err(ValidationError::new("page_size_too_large"));
        }
        Ok(())
    }

    #[derive(Debug, Deserialize, Validate, summer_web::ValidatorContext)]
    #[validate(context = PageRules)]
    struct Paginator {
        #[validate(custom(function = "validate_page_size", use_context))]
        page_size: usize,
    }

    async fn paginator_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::Json<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn paginator_query_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Query<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn paginator_path_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Path<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn paginator_form_handler(
        payload: summer_web::validation::validator::ValidatorEx<summer_web::axum::extract::Form<Paginator>>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "page_size": payload.page_size,
        }))
    }

    async fn router_with_rules(rules: Option<PageRules>) -> Router {
        let mut app = summer::app::AppBuilder::default();
        if let Some(rules) = rules {
            app.add_component(rules);
        }
        let app = app.build().await.expect("app build");

        let router = Router::new()
            .route("/paginator", axum::routing::post(paginator_handler))
            .route("/paginator-query", axum::routing::get(paginator_query_handler))
            .route("/paginator-path/{page_size}", axum::routing::get(paginator_path_handler))
            .route("/paginator-form", axum::routing::post(paginator_form_handler));

        router.layer(Extension(AppState { app }))
    }

    fn json_request(body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/paginator")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request")
    }

    async fn read_problem(response: axum::response::Response) -> ProblemDetails {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("problem details json")
    }

    fn query_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/paginator-query?page_size={page_size}"))
            .body(Body::empty())
            .expect("request")
    }

    fn path_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/paginator-path/{page_size}"))
            .body(Body::empty())
            .expect("request")
    }

    fn form_request(page_size: usize) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/paginator-form")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("page_size={page_size}")))
            .expect("request")
    }

    #[tokio::test]
    async fn validator_json_with_args_uses_registered_context() {
        let response = router_with_rules(Some(PageRules { max_page_size: 100 }))
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn validator_json_with_args_validation_error_maps_to_body_violation() {
        let response = router_with_rules(Some(PageRules { max_page_size: 10 }))
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations.len(), 1);
        assert_eq!(problem.violations[0].field, "page_size");
        assert_eq!(problem.violations[0].location, ViolationLocation::Body);
    }

    #[tokio::test]
    async fn validator_json_with_args_missing_context_component_returns_server_error() {
        let response = router_with_rules(None)
            .await
            .oneshot(json_request(serde_json::json!({ "page_size": 20 })))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn validator_query_with_args_validation_error_maps_to_query_violation() {
        let response = router_with_rules(Some(PageRules { max_page_size: 10 }))
            .await
            .oneshot(query_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Query);
    }

    #[tokio::test]
    async fn validator_path_with_args_validation_error_maps_to_path_violation() {
        let response = router_with_rules(Some(PageRules { max_page_size: 10 }))
            .await
            .oneshot(path_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Path);
    }

    #[tokio::test]
    async fn validator_form_with_args_validation_error_maps_to_form_violation() {
        let response = router_with_rules(Some(PageRules { max_page_size: 10 }))
            .await
            .oneshot(form_request(20))
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let problem = read_problem(response).await;
        assert_eq!(problem.violations[0].location, ViolationLocation::Form);
    }

}
