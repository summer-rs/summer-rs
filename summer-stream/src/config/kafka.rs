#![allow(warnings, unused)]

use super::OptionsFiller;
use crate::config::ConsumerModeRef;
use schemars::JsonSchema;
use sea_streamer::kafka::{
    KafkaConnectOptions, KafkaConsumerOptions, KafkaProducerOptions, SaslMechanism,
};
use sea_streamer::{ConnectOptions as ConnectOptionsTrait, ConsumerMode};
use serde::Deserialize;
use std::time::Duration;

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
pub struct KafkaOptions {
    connect: Option<ConnectOptions>,
    producer: Option<ProducerOptions>,
    consumer: Option<ConsumerOptions>,
}

impl OptionsFiller for KafkaOptions {
    type ConnectOptsType = KafkaConnectOptions;
    type ConsumerOptsType = KafkaConsumerOptions;
    type ProducerOptsType = KafkaProducerOptions;

    fn fill_connect_options(&self, opts: &mut Self::ConnectOptsType) {
        if let Some(conn) = &self.connect {
            if let Some(timeout) = conn.timeout {
                let _ = opts.set_timeout(timeout);
            }
            if let Some(sasl_options) = &conn.sasl_options {
                let mut so = sea_streamer::kafka::SaslOptions::new(sasl_options.mechanism);
                if let Some(usr) = &sasl_options.username {
                    so = so.username(usr);
                }
                if let Some(pwd) = &sasl_options.password {
                    so = so.password(pwd);
                }
                opts.set_sasl_options(so);
            }
            if let Some(protocol) = &conn.security_protocol {
                match protocol {
                    SecurityProtocol::Plaintext => {
                        opts.set_security_protocol(
                            sea_streamer::kafka::SecurityProtocol::Plaintext,
                        );
                    }
                    SecurityProtocol::SaslPlaintext => {
                        opts.set_security_protocol(
                            sea_streamer::kafka::SecurityProtocol::SaslPlaintext,
                        );
                    }
                    SecurityProtocol::Ssl => {
                        opts.set_security_protocol(sea_streamer::kafka::SecurityProtocol::Ssl);
                    }
                    SecurityProtocol::SaslSsl => {
                        opts.set_security_protocol(sea_streamer::kafka::SecurityProtocol::SaslSsl);
                    }
                }
            }
            for pair in conn.custom_options.clone() {
                opts.add_custom_option(pair.0, pair.1);
            }
        }
    }

    fn fill_consumer_options(&self, opts: &mut Self::ConsumerOptsType) {
        if let Some(consumer) = &self.consumer {
            if let Some(session_timeout) = consumer.session_timeout {
                opts.set_session_timeout(session_timeout);
            }
            if let Some(auto_offset_reset) = &consumer.auto_offset_reset {
                match auto_offset_reset {
                    AutoOffsetReset::Earliest => {
                        opts.set_auto_offset_reset(sea_streamer::kafka::AutoOffsetReset::Earliest);
                    }
                    AutoOffsetReset::NoReset => {
                        opts.set_auto_offset_reset(sea_streamer::kafka::AutoOffsetReset::NoReset);
                    }
                    AutoOffsetReset::Latest => {
                        opts.set_auto_offset_reset(sea_streamer::kafka::AutoOffsetReset::Latest);
                    }
                }
            }
            if let Some(enable_auto_commit) = consumer.enable_auto_commit {
                opts.set_enable_auto_commit(enable_auto_commit);
            }
            if let Some(auto_commit_interval) = consumer.auto_commit_interval {
                opts.set_auto_commit_interval(auto_commit_interval);
            }
            if let Some(enable_auto_offset_store) = consumer.enable_auto_offset_store {
                opts.set_enable_auto_offset_store(enable_auto_offset_store);
            }
            for pair in consumer.custom_options.clone() {
                opts.add_custom_option(pair.0, pair.1);
            }
        }
    }

    fn fill_producer_options(&self, opts: &mut Self::ProducerOptsType) {
        if let Some(producer) = &self.producer {
            if let Some(compress_type) = &producer.compression_type {
                match compress_type {
                    CompressionType::Gzip => {
                        opts.set_compression_type(sea_streamer::kafka::CompressionType::Gzip)
                    }
                    CompressionType::Lz4 => {
                        opts.set_compression_type(sea_streamer::kafka::CompressionType::Lz4)
                    }
                    CompressionType::Snappy => {
                        opts.set_compression_type(sea_streamer::kafka::CompressionType::Snappy)
                    }
                    CompressionType::Zstd => {
                        opts.set_compression_type(sea_streamer::kafka::CompressionType::Zstd)
                    }
                    CompressionType::None => {
                        opts.set_compression_type(sea_streamer::kafka::CompressionType::None)
                    }
                };
            }
            if let Some(transaction_timeout) = producer.transaction_timeout {
                opts.set_transaction_timeout(transaction_timeout);
            }
            for pair in producer.custom_options.clone() {
                opts.add_custom_option(pair.0, pair.1);
            }
        }
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
    timeout: Option<Duration>,
    security_protocol: Option<SecurityProtocol>,
    sasl_options: Option<SaslOptions>,
    #[serde(default)]
    custom_options: Vec<(String, String)>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
struct SaslOptions {
    #[serde(with = "SaslMechanismRef")]
    mechanism: SaslMechanism,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(remote = "SaslMechanism")]
enum SaslMechanismRef {
    Plain,
    Gssapi,
    ScramSha256,
    ScramSha512,
    Oauthbearer,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
enum SecurityProtocol {
    Plaintext,
    Ssl,
    SaslPlaintext,
    SaslSsl,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ConsumerOptions {
    #[serde(with = "ConsumerModeRef")]
    mode: ConsumerMode,
    group_id: Option<String>,
    session_timeout: Option<Duration>,
    auto_offset_reset: Option<AutoOffsetReset>,
    enable_auto_commit: Option<bool>,
    auto_commit_interval: Option<Duration>,
    enable_auto_offset_store: Option<bool>,
    #[serde(default)]
    custom_options: Vec<(String, String)>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
enum AutoOffsetReset {
    /// Automatically reset the offset to the earliest offset.
    Earliest,
    /// Automatically reset the offset to the latest offset.
    Latest,
    /// Throw exception to the consumer if no previous offset is found for the consumer's group.
    NoReset,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ProducerOptions {
    compression_type: Option<CompressionType>,
    transaction_timeout: Option<Duration>,
    #[serde(default)]
    custom_options: Vec<(String, String)>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
enum CompressionType {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}