#![allow(warnings, unused)]

use super::OptionsFiller;
use crate::config::ConsumerModeRef;
use schemars::JsonSchema;
use sea_streamer::redis::{
    AutoCommit, AutoStreamReset, PseudoRandomSharder, RedisConnectOptions, RedisConsumerOptions,
    RedisProducerOptions, RoundRobinSharder, ShardOwnership,
};
use sea_streamer::{ConnectOptions as ConnectOptionsTrait, ConsumerId, ConsumerMode};
use serde::Deserialize;
use std::time::Duration;

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
pub struct RedisOptions {
    connect: Option<ConnectOptions>,
    producer: Option<ProducerOptions>,
    consumer: Option<ConsumerOptions>,
}

impl OptionsFiller for RedisOptions {
    type ConnectOptsType = RedisConnectOptions;
    type ConsumerOptsType = RedisConsumerOptions;
    type ProducerOptsType = RedisProducerOptions;

    fn fill_connect_options(&self, opts: &mut Self::ConnectOptsType) {
        if let Some(connect) = &self.connect {
            opts.set_db(connect.db);
            opts.set_username(connect.username.clone());
            opts.set_password(connect.password.clone());
            opts.set_enable_cluster(connect.enable_cluster);
            if let Some(disable_hostname_verification) = connect.disable_hostname_verification {
                opts.set_disable_hostname_verification(disable_hostname_verification);
            }
            if let Some(timeout) = connect.timeout {
                let _ = opts.set_timeout(timeout);
            }
        }
    }

    fn fill_consumer_options(&self, opts: &mut Self::ConsumerOptsType) {
        if let Some(consumer) = &self.consumer {
            if let Some(consumer_id) = &consumer.consumer_id {
                opts.set_consumer_id(ConsumerId::new(consumer_id));
            }
            opts.set_consumer_timeout(consumer.consumer_timeout);
            opts.set_auto_stream_reset(consumer.auto_stream_reset);
            opts.set_auto_commit(consumer.auto_commit);
            opts.set_auto_commit_interval(consumer.auto_commit_interval);
            opts.set_auto_stream_reset(consumer.auto_stream_reset);
            opts.set_auto_claim_interval(consumer.auto_claim_interval);
            opts.set_auto_claim_idle(consumer.auto_claim_idle);
            opts.set_batch_size(consumer.batch_size);
            opts.set_shard_ownership(consumer.shard_ownership);
            opts.set_mkstream(consumer.mkstream);
        }
    }

    fn fill_producer_options(&self, opts: &mut Self::ProducerOptsType) {
        if let Some(ProducerOptions {
            sharder_algo: Some(sharder_algo),
            num_shards,
        }) = &self.producer
        {
            match sharder_algo {
                Sharder::PseudoRandom => {
                    opts.set_sharder(PseudoRandomSharder::new(*num_shards as u64))
                }
                Sharder::RoundRobin => opts.set_sharder(RoundRobinSharder::new(*num_shards)),
            };
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

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ConnectOptions {
    db: u32,
    username: Option<String>,
    password: Option<String>,
    timeout: Option<Duration>,
    #[serde(default)]
    enable_cluster: bool,
    disable_hostname_verification: Option<bool>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
struct ConsumerOptions {
    #[serde(with = "ConsumerModeRef")]
    mode: ConsumerMode,
    group_id: Option<String>,
    consumer_id: Option<String>,
    consumer_timeout: Option<Duration>,
    #[serde(with = "AutoStreamResetRef")]
    auto_stream_reset: AutoStreamReset,
    #[serde(with = "AutoCommitRef")]
    auto_commit: AutoCommit,
    auto_commit_delay: Duration,
    auto_commit_interval: Duration,
    auto_claim_interval: Option<Duration>,
    auto_claim_idle: Duration,
    batch_size: usize,
    #[serde(with = "ShardOwnershipRef")]
    shard_ownership: ShardOwnership,
    mkstream: bool,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(remote = "AutoStreamReset")]
/// Where to start streaming from if there is no priori state.
enum AutoStreamResetRef {
    /// Use `0` as ID, which is the earliest message.
    Earliest,
    /// Use `$` as ID, which is the latest message.
    Latest,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(remote = "AutoCommit")]
/// The auto ack / commit mechanism.
enum AutoCommitRef {
    /// `XREAD` with `NOACK`. This acknowledges messages as soon as they are fetched.
    /// In the event of service restart, this will likely result in messages being skipped.
    Immediate,
    /// Auto ack and commit, but only after `auto_commit_delay` has passed since messages are read.
    ///
    /// This is the default option, because users don't have to do anything extra. This also mimicks Kafka clients.
    Delayed,
    /// Do not auto ack, but continually commit acked messages to the server as new messages are read.
    /// The consumer will not commit more often than `auto_commit_interval`.
    /// You have to call [`RedisConsumer::ack`](crate::RedisConsumer::ack) manually.
    ///
    /// This is the recommended option for achieving 'at-least-once' semantics.
    Rolling,
    /// Never auto ack or commit.
    /// You have to call [`RedisConsumer::ack`](crate::RedisConsumer::ack) and [`RedisConsumer::commit`](crate::RedisConsumer::commit) manually.
    Disabled,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
#[serde(remote = "ShardOwnership")]
/// The shard ownership model.
enum ShardOwnershipRef {
    /// Consumers in the same group share the same shard
    Shared,
    /// Consumers claim ownership of a shard
    ///
    /// > This feature is still WIP
    Owned,
}

#[derive(Default, Debug, Clone, JsonSchema, Deserialize)]
struct ProducerOptions {
    sharder_algo: Option<Sharder>,
    #[serde(default = "default_num_shards")]
    num_shards: u32,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
enum Sharder {
    RoundRobin,
    PseudoRandom,
}

fn default_num_shards() -> u32 {
    16
}
