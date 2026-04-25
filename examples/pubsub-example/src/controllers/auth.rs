use summer_web::nest;

#[nest("/auth")]
pub mod auth {
    use summer_sqlx::ConnectPool;
    use summer_web::axum::response::IntoResponse;
    use summer_web::extractor::{Component, Config};
    use summer_web::get;
    use summer_pubsub::{PubSubConfig, PubSubProducer};
    use summer_pubsub::model::Message;
    use summer::tracing;
    use summer_web::axum::http::StatusCode;
    
    #[get("/signup")]
    pub async fn signup(
        Config(_pubsub_config): Config<PubSubConfig>, // You can import other configs here if you need
        Component(_pool): Component<ConnectPool>,
        Component(pubsub): Component<PubSubProducer>,
    ) -> impl IntoResponse {
        tracing::info!("signup");

        tracing::info!("You can use other components and configs here");


        let user_id = "1234567890";
        let email = "test@test.com";
        let created_at = "2026-04-25T12:00:00Z";

        let message = Message::new()
            .set_data(user_id.as_bytes())
            .set_attributes([
                ("user_id".to_string(), user_id.to_string()),
                ("email".to_string(), email.to_string()),
                ("created_at".to_string(), created_at.to_string()),
            ])
            .set_ordering_key(user_id.to_string());
        // publish a message to the pubsub topic "users_created"
        let something = pubsub.publish("users_created", message).await;
        tracing::info!(?something, "published signup message");
        (StatusCode::OK, "Signup successful").into_response()
    }
}