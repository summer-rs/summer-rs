use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;

summer::submit_config_schema!("redis", RedisConfig);

#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "redis"]
pub struct RedisConfig {
    /// The URI for connecting to the Redis server. For example:
    /// <redis://127.0.0.1/>
    pub uri: String,

    /// The new connection will time out operations after `response_timeout` has passed.
    pub response_timeout: Option<u64>,

    /// Each connection attempt to the server will time out after `connection_timeout`.
    pub connection_timeout: Option<u64>,

    /// number_of_retries times, with an exponentially increasing delay
    pub number_of_retries: Option<usize>,

    /// The resulting duration is calculated by taking the base to the `n`-th power,
    /// where `n` denotes the number of past attempts.
    pub exponent_base: Option<f32>,

    /// Apply a maximum delay between connection attempts. The delay between attempts won't be longer than max_delay milliseconds.
    pub max_delay: Option<u64>,
}
