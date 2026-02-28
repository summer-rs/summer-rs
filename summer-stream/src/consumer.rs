use crate::{
    handler::{BoxedHandler, Handler},
    Streamer,
};
use anyhow::Context;
#[cfg(feature = "file")]
use sea_streamer::file::FileConsumerOptions;
#[cfg(feature = "kafka")]
use sea_streamer::kafka::KafkaConsumerOptions;
#[cfg(feature = "redis")]
use sea_streamer::redis::RedisConsumerOptions;
#[cfg(feature = "stdio")]
use sea_streamer::stdio::StdioConsumerOptions;
use sea_streamer::Consumer as _;
use sea_streamer::{ConsumerGroup, ConsumerMode, ConsumerOptions, SeaConsumer, SeaConsumerOptions};
use summer::{app::App, error::Result};
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Default)]
pub struct Consumers(Vec<Consumer>);

impl Consumers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_consumer(mut self, consumer: Consumer) -> Self {
        self.0.push(consumer);
        self
    }

    pub(crate) fn merge(&mut self, consumers: Self) {
        for consumer in consumers.0 {
            self.0.push(consumer);
        }
    }
}

impl Deref for Consumers {
    type Target = Vec<Consumer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Consumer {
    pub(crate) stream_keys: &'static [&'static str],
    pub(crate) opts: ConsumerOpts,
    pub(crate) handler: BoxedHandler,
}

#[derive(Default, Clone)]
pub struct ConsumerOpts(pub(crate) SeaConsumerOptions);

impl Consumer {
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> ConsumerOpts {
        ConsumerOpts(Default::default())
    }

    pub fn mode(mode: ConsumerMode) -> ConsumerOpts {
        ConsumerOpts(SeaConsumerOptions::new(mode))
    }

    pub(crate) async fn new_instance(&self, streamer: &Streamer) -> Result<ConsumerInstance> {
        let consumer = streamer
            .create_consumer(self.stream_keys, self.opts.clone())
            .await?;
        Ok(ConsumerInstance {
            consumer,
            handler: self.handler.clone(),
        })
    }
}

impl ConsumerOpts {
    pub fn group_id(mut self, group_id: &'static str) -> Self {
        let _ = self.0.set_consumer_group(ConsumerGroup::new(group_id));
        self
    }
    #[cfg(feature = "kafka")]
    pub fn kafka_consumer_options<F>(mut self, func: F) -> Self
    where
        F: FnOnce(&mut KafkaConsumerOptions) + Send + Sync + 'static,
    {
        self.0.set_kafka_consumer_options(func);
        self
    }
    #[cfg(feature = "redis")]
    pub fn redis_consumer_options<F>(mut self, func: F) -> Self
    where
        F: FnOnce(&mut RedisConsumerOptions) + Send + Sync + 'static,
    {
        self.0.set_redis_consumer_options(func);
        self
    }
    #[cfg(feature = "stdio")]
    pub fn stdio_consumer_options<F>(mut self, func: F) -> Self
    where
        F: FnOnce(&mut StdioConsumerOptions) + Send + Sync + 'static,
    {
        self.0.set_stdio_consumer_options(func);
        self
    }
    #[cfg(feature = "file")]
    pub fn file_consumer_options<F>(mut self, func: F) -> Self
    where
        F: FnOnce(&mut FileConsumerOptions) + Send + Sync + 'static,
    {
        self.0.set_file_consumer_options(func);
        self
    }

    pub fn consume<H, A>(self, stream_keys: &'static [&'static str], handler: H) -> Consumer
    where
        H: Handler<A> + Sync,
        A: 'static,
    {
        Consumer {
            handler: BoxedHandler::from_handler(handler),
            stream_keys,
            opts: self,
        }
    }
}

pub(crate) struct ConsumerInstance {
    consumer: SeaConsumer,
    handler: BoxedHandler,
}

impl ConsumerInstance {
    pub async fn schedule(self, app: Arc<App>) -> Result<String> {
        let ConsumerInstance { consumer, handler } = self;
        loop {
            let message = consumer.next().await.context("consumer poll msg failed")?;
            BoxedHandler::call(handler.clone(), message, app.clone()).await;
        }
    }
}
