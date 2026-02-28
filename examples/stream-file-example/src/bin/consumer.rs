use summer::tracing;
use summer::App;
use summer_stream::consumer::Consumers;
use summer_stream::extractor::Json;
use summer_stream::file::AutoStreamReset;
use summer_stream::handler::TypedConsumer;
use summer_stream::stream_listener;
use summer_stream::{file::FileConsumerOptions, StreamConfigurator, StreamPlugin};
use stream_file_example::Payload;

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
    "topic2",
    file_consumer_options = fill_file_consumer_options
)]
async fn listen_topic_do_something(Json(payload): Json<Payload>) {
    tracing::info!("{:#?}", payload);
}

fn fill_file_consumer_options(opts: &mut FileConsumerOptions) {
    opts.set_auto_stream_reset(AutoStreamReset::Earliest);
}
