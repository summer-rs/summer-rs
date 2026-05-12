use summer::tracing;
use summer::App;
use summer_stream::consumer::Consumers;
use summer_stream::extractor::{Json, StreamKey};
use summer_stream::handler::TypedConsumer;
use summer_stream::stream_listener;
use summer_stream::{kafka::KafkaConsumerOptions, StreamConfigurator, StreamPlugin};
use stream_kafka_example::Payload;

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(StreamPlugin)
        .add_consumer(consumers())
        .run()
        .await
}

fn consumers() -> Consumers {
    Consumers::new().typed_consumer(listen_topic_do_something)
}

#[stream_listener(
    "topic",
    kafka_consumer_options = fill_kafka_consumer_options
)]
async fn listen_topic_do_something(topic: StreamKey, Json(payload): Json<Payload>) {
    tracing::info!("received msg from topic#{}: {:#?}", topic, payload);
}

fn fill_kafka_consumer_options(opts: &mut KafkaConsumerOptions) {
    opts.set_enable_auto_offset_store(true);
}
