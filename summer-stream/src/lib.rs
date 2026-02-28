//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-stream)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;
pub mod consumer;
pub mod extractor;
pub mod handler;

pub use consumer::{ConsumerOpts, Consumers};
pub use sea_streamer;
#[cfg(feature = "file")]
pub use sea_streamer::file;
#[cfg(feature = "kafka")]
pub use sea_streamer::kafka;
#[cfg(feature = "redis")]
pub use sea_streamer::redis;
#[cfg(feature = "stdio")]
pub use sea_streamer::stdio;
/////////////////stream-macros/////////////////////
pub use summer_macros::stream_listener;

use anyhow::Context;
use config::StreamConfig;
use sea_streamer::{
    Buffer, MessageHeader, Producer as _, SeaConsumer, SeaProducer, SeaStreamer, StreamKey,
    Streamer as _, StreamerUri,
};
#[cfg(feature = "json")]
use serde::Serialize;
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::error::Result;
use summer::plugin::component::ComponentRef;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry};
use summer::{
    app::{App, AppBuilder},
    plugin::Plugin,
};
use std::ops::Deref;
use std::{str::FromStr, sync::Arc};

pub trait StreamConfigurator {
    fn add_consumer(&mut self, consumers: Consumers) -> &mut Self;
}

impl StreamConfigurator for AppBuilder {
    fn add_consumer(&mut self, new_consumers: Consumers) -> &mut Self {
        if let Some(consumers) = self.get_component_ref::<Consumers>() {
            unsafe {
                let raw_ptr = ComponentRef::into_raw(consumers);
                let consumers = &mut *(raw_ptr as *mut Consumers);
                consumers.merge(new_consumers);
            }
            self
        } else {
            self.add_component(new_consumers)
        }
    }
}

pub struct StreamPlugin;

#[async_trait]
impl Plugin for StreamPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<StreamConfig>()
            .expect("sea-streamer plugin config load failed");

        let streamer = Streamer::new(config).await.expect("create streamer failed");

        if let Some(consumers) = app.get_component_ref::<Consumers>() {
            for consumer in consumers.deref().iter() {
                let consumer_instance = consumer
                    .new_instance(&streamer)
                    .await
                    .expect("create customer instance failed");
                app.add_scheduler(|app: Arc<App>| Box::new(consumer_instance.schedule(app)));
                tracing::info!(
                    "register scheduler for \"{:?}\" stream consumer",
                    consumer.stream_keys
                );
            }
        } else {
            tracing::info!("not consumer be registry");
        }
        let producer = streamer
            .create_generic_producer()
            .await
            .expect("create producer failed");

        app.add_component(producer);
    }
}

pub struct Streamer {
    streamer: SeaStreamer,
    config: StreamConfig,
}

impl Streamer {
    async fn new(config: StreamConfig) -> Result<Self> {
        let uri = StreamerUri::from_str(config.uri.as_str())
            .with_context(|| format!("parse stream server \"{}\" failed", config.uri))?;

        let streamer = SeaStreamer::connect(uri, config.connect_options())
            .await
            .with_context(|| format!("connect stream server \"{}\" failed", config.uri))?;

        Ok(Self { streamer, config })
    }

    async fn create_consumer(
        &self,
        stream_keys: &'static [&'static str],
        opts: ConsumerOpts,
    ) -> Result<SeaConsumer> {
        let consumer_options = self.config.new_consumer_options(opts);
        let mut consumer_stream_keys = Vec::with_capacity(stream_keys.len());
        for key in stream_keys {
            consumer_stream_keys.push(
                StreamKey::new(*key)
                    .with_context(|| format!("consumer stream key \"{key}\" is valid"))?,
            );
        }
        Ok(self
            .streamer
            .create_consumer(&consumer_stream_keys, consumer_options)
            .await
            .with_context(|| format!("create stream consumer failed: {stream_keys:?}"))?)
    }

    async fn create_generic_producer(&self) -> Result<Producer> {
        let producer_options = self.config.new_producer_options();
        let producer = self
            .streamer
            .create_generic_producer(producer_options)
            .await
            .context("create stream generic producer failed")?;
        Ok(Producer::new(producer))
    }
}

#[derive(Clone)]
pub struct Producer(Arc<SeaProducer>);

impl Producer {
    fn new(producer: SeaProducer) -> Self {
        Self(Arc::new(producer))
    }

    #[cfg(feature = "json")]
    pub async fn send_json<T: Serialize>(
        &self,
        stream_key: &str,
        payload: T,
    ) -> Result<MessageHeader> {
        let json = serde_json::to_string(&payload).context("json serialize failed")?;
        self.send_to(stream_key, json.as_str()).await
    }

    pub async fn send_to<S: Buffer>(&self, stream_key: &str, payload: S) -> Result<MessageHeader> {
        let producer_stream_key = StreamKey::new(stream_key)
            .with_context(|| format!("producer stream key \"{stream_key}\" is valid"))?;

        let header = self
            .0
            .send_to(&producer_stream_key, payload)
            .with_context(|| format!("send to stream key failed:{stream_key}"))?
            .await
            .with_context(|| {
                format!("await response for sending stream key failed:{stream_key}")
            })?;

        Ok(header)
    }
}
