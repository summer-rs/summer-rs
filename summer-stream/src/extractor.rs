pub use sea_streamer::Message;
pub use sea_streamer::MessageHeader;
pub use sea_streamer::SeaMessage;
pub use sea_streamer::SeqNo;
pub use sea_streamer::ShardId;
pub use sea_streamer::SharedMessage;
pub use sea_streamer::StreamKey;
pub use sea_streamer::Timestamp;

use summer::app::App;
use summer::config::ConfigRegistry;
use summer::config::Configurable;
use summer::extractor::Component;
use summer::extractor::Config;
use summer::plugin::ComponentRegistry;

pub trait FromMsg {
    fn from_msg(msg: &SeaMessage, app: &App) -> Self;
}

impl FromMsg for StreamKey {
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        msg.stream_key()
    }
}

impl FromMsg for SeqNo {
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        msg.sequence()
    }
}

impl FromMsg for ShardId {
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        msg.shard_id()
    }
}

impl FromMsg for Timestamp {
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        msg.timestamp()
    }
}

impl FromMsg for MessageHeader {
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        MessageHeader::new(
            msg.stream_key(),
            msg.shard_id(),
            msg.sequence(),
            msg.timestamp(),
        )
    }
}

impl FromMsg for SharedMessage {
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        msg.to_owned()
    }
}

impl<T> FromMsg for Component<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn from_msg(_msg: &SeaMessage, app: &App) -> Self {
        match app.get_component_ref::<T>() {
            Some(component) => Component(T::clone(&component)),
            None => panic!(
                "There is no component of `{}` type",
                std::any::type_name::<T>()
            ),
        }
    }
}

impl<T> FromMsg for Config<T>
where
    T: serde::de::DeserializeOwned + Configurable,
{
    fn from_msg(_msg: &SeaMessage, app: &App) -> Self {
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

#[cfg(feature = "json")]
pub struct Json<T>(pub T);

#[cfg(feature = "json")]
impl<T> FromMsg for Json<T>
where
    T: serde::de::DeserializeOwned,
{
    fn from_msg(msg: &SeaMessage, _app: &App) -> Self {
        let value = msg
            .message()
            .deserialize_json()
            .expect("stream message parse as json failed");
        Json(value)
    }
}
