use summer::extractor::Component;
use summer::extractor::Config;
use summer_pubsub::PubSubConfig;
use summer_pubsub::pubsub_listener;
use summer_pubsub::Message;
use summer::tracing;
use summer_sqlx::ConnectPool;

#[pubsub_listener("users_created_listener")]
async fn on_users_created_message(
    Config(_pubsub_config): Config<PubSubConfig>, // You can import other configs here if you need
    Component(_pool): Component<ConnectPool>,
    msg: Message,
) {
    let payload = String::from_utf8(msg.data.to_vec()).unwrap_or("<binary>".to_string());

    let user_id = msg.attributes.get("user_id").map(String::as_str).unwrap_or("unknown");
    let email = msg.attributes.get("email").map(String::as_str).unwrap_or("unknown");
    let created_at = msg.attributes.get("created_at").map(String::as_str).unwrap_or("unknown");

    tracing::info!(
        message_id = %msg.message_id,
        len = msg.data.len(),
        payload,
        user_id,
        email,
        created_at,
        "pubsub users_created (received)"
    );
}
