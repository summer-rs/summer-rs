use crate::handler::PubSubEnvelope;
use crate::message::Message;
use summer::app::App;
use summer::config::Configurable;
use summer::config::ConfigRegistry;
use summer::extractor::Component;
use summer::extractor::Config;
use summer::plugin::ComponentRegistry;

pub trait FromPubSubMsg: Sized {
    fn from_pubsub(
        grpc: &google_cloud_pubsub::model::Message,
        env: &PubSubEnvelope,
        app: &App,
    ) -> Self;
}

impl FromPubSubMsg for Message {
    fn from_pubsub(
        grpc: &google_cloud_pubsub::model::Message,
        env: &PubSubEnvelope,
        _app: &App,
    ) -> Self {
        Message::new(
            grpc.message_id.clone(),
            grpc.data.clone(),
            grpc.attributes.clone(),
            env.ack.clone(),
        )
    }
}

impl<T> FromPubSubMsg for Component<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn from_pubsub(_grpc: &google_cloud_pubsub::model::Message, _env: &PubSubEnvelope, app: &App) -> Self {
        match app.get_component_ref::<T>() {
            Some(component) => Component(T::clone(&component)),
            None => panic!(
                "There is no component of `{}` type",
                std::any::type_name::<T>()
            ),
        }
    }
}

impl<T> FromPubSubMsg for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
{
    fn from_pubsub(_grpc: &google_cloud_pubsub::model::Message, _env: &PubSubEnvelope, app: &App) -> Self {
        match app.get_config::<T>() {
            Ok(config) => Config(config),
            Err(e) => panic!(
                "get config failed for typeof {}: {}",
                std::any::type_name::<T>(),
                e
            ),
        }
    }
}
