#![allow(warnings, unused)]

use super::OptionsFiller;
use crate::config::ConsumerModeRef;
use schemars::JsonSchema;
use sea_streamer::{
    file::{AutoStreamReset, FileConnectOptions, FileConsumerOptions, FileProducerOptions},
    ConsumerGroup, ConsumerMode, ConsumerOptions as ConsumerOptionsTrait, SeaConsumerOptions,
};
use serde::Deserialize;

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
pub struct FileOptions {
    connect: Option<ConnectOptions>,
    producer: Option<ProducerOptions>,
    consumer: Option<ConsumerOptions>,
}

impl OptionsFiller for FileOptions {
    type ConnectOptsType = FileConnectOptions;
    type ConsumerOptsType = FileConsumerOptions;
    type ProducerOptsType = FileProducerOptions;

    fn fill_connect_options(&self, opts: &mut Self::ConnectOptsType) {
        if let Some(connect) = &self.connect {
            match connect.create_file {
                CreateFileOption::Always => opts.set_create_only(true),
                CreateFileOption::Never => opts.set_create_if_not_exists(false),
                CreateFileOption::CreateIfNotExists => opts.set_create_if_not_exists(true),
            };
            opts.set_end_with_eos(connect.end_with_eos);
            if let Some(beacon_interval) = connect.beacon_interval {
                let _ = opts.set_beacon_interval(beacon_interval * 1024);
            }
            if let Some(file_size_limit) = connect.file_size_limit {
                opts.set_file_size_limit(file_size_limit);
            }
            if let Some(prefetch_message) = connect.prefetch_message {
                opts.set_prefetch_message(prefetch_message);
            }
        } else {
            opts.create_if_not_exists();
        }
    }

    fn fill_consumer_options(&self, opts: &mut Self::ConsumerOptsType) {
        if let Some(consumer) = &self.consumer {
            opts.set_auto_stream_reset(consumer.auto_stream_reset);
            opts.set_live_streaming(consumer.live_streaming);
        }
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

#[derive(Debug, Clone, JsonSchema, Deserialize)]
struct ConnectOptions {
    create_file: CreateFileOption,
    #[serde(default)]
    end_with_eos: bool,
    beacon_interval: Option<u32>,
    file_size_limit: Option<u64>,
    prefetch_message: Option<usize>,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
enum CreateFileOption {
    /// File must already exists
    Never,
    #[default]
    CreateIfNotExists,
    /// Fail if the file already exists
    Always,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
struct ConsumerOptions {
    #[serde(with = "ConsumerModeRef")]
    mode: ConsumerMode,
    group_id: Option<String>,
    #[serde(with = "AutoStreamResetRef")]
    auto_stream_reset: AutoStreamReset,
    live_streaming: bool,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(remote = "AutoStreamReset")]
enum AutoStreamResetRef {
    Earliest,
    Latest,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ProducerOptions {}
