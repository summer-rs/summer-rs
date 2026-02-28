#![allow(warnings, unused)]

use super::OptionsFiller;
use crate::config::ConsumerModeRef;
use schemars::JsonSchema;
use sea_streamer::{
    stdio::{StdioConnectOptions, StdioConsumerOptions, StdioProducerOptions},
    ConsumerMode,
};
use serde::Deserialize;

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
pub struct StdioOptions {
    connect: Option<ConnectOptions>,
    producer: Option<ProducerOptions>,
    consumer: Option<ConsumerOptions>,
}

impl OptionsFiller for StdioOptions {
    type ConnectOptsType = StdioConnectOptions;
    type ConsumerOptsType = StdioConsumerOptions;
    type ProducerOptsType = StdioProducerOptions;

    fn fill_connect_options(&self, opts: &mut Self::ConnectOptsType) {
        if let Some(connect) = &self.connect {
            opts.set_loopback(connect.loopback);
        }
    }

    fn fill_consumer_options(&self, _opts: &mut Self::ConsumerOptsType) {
        // no ops
    }

    fn fill_producer_options(&self, _opts: &mut Self::ProducerOptsType) {
        // no ops
    }

    fn default_consumer_mode(&self) -> Option<&ConsumerMode> {
        match &self.consumer {
            Some(consumer) => Some(&consumer.mode),
            None => None,
        }
    }

    fn default_consumer_group_id(&self) -> Option<String> {
        match &self.consumer {
            Some(consumer) => consumer.group_id.clone(),
            None => None,
        }
    }
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ConnectOptions {
    loopback: bool,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ConsumerOptions {
    #[serde(with = "ConsumerModeRef")]
    mode: ConsumerMode,
    group_id: Option<String>,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ProducerOptions {}
