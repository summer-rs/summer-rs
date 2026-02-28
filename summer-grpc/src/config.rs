use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

summer::submit_config_schema!("grpc", GrpcConfig);

/// summer-grpc Config
#[derive(Debug, Configurable, JsonSchema, Deserialize)]
#[config_prefix = "grpc"]
pub struct GrpcConfig {
    #[serde(default = "default_binding")]
    pub(crate) binding: IpAddr,
    #[serde(default = "default_port")]
    pub(crate) port: u16,

    /// Set the concurrency limit applied to on requests inbound per connection.
    pub(crate) concurrency_limit_per_connection: Option<usize>,

    /// Set a timeout on for all request handlers.
    pub(crate) timeout: Option<Duration>,

    /// Sets the [`SETTINGS_INITIAL_WINDOW_SIZE`][spec] option for HTTP2
    /// stream-level flow control.
    ///
    /// Default is 65,535
    ///
    /// [spec]: https://httpwg.org/specs/rfc9113.html#InitialWindowSize
    pub(crate) initial_stream_window_size: Option<u32>,

    /// Sets the max connection-level flow control for HTTP2
    pub(crate) initial_connection_window_size: Option<u32>,

    /// Sets the [`SETTINGS_MAX_CONCURRENT_STREAMS`][spec] option for HTTP2
    /// connections.
    pub(crate) max_concurrent_streams: Option<u32>,

    /// Sets the maximum time option in milliseconds that a connection may exist
    ///
    /// Default is no limit (`None`).
    pub(crate) max_connection_age: Option<Duration>,

    /// Set whether HTTP2 Ping frames are enabled on accepted connections.
    ///
    /// If `None` is specified, HTTP2 keepalive is disabled, otherwise the duration
    /// specified will be the time interval between HTTP2 Ping frames.
    /// The timeout for receiving an acknowledgement of the keepalive ping
    /// can be set with [`Server::http2_keepalive_timeout`].
    ///
    /// Default is no HTTP2 keepalive (`None`)
    pub(crate) http2_keepalive_interval: Option<Duration>,

    /// Sets a timeout for receiving an acknowledgement of the keepalive ping.
    ///
    /// If the ping is not acknowledged within the timeout, the connection will be closed.
    /// Does nothing if http2_keep_alive_interval is disabled.
    ///
    /// Default is 20 seconds.
    pub(crate) http2_keepalive_timeout: Option<Duration>,

    /// Sets whether to use an adaptive flow control. Defaults to false.
    /// Enabling this will override the limits set in http2_initial_stream_window_size and
    /// http2_initial_connection_window_size.
    pub(crate) http2_adaptive_window: Option<bool>,

    /// Configures the maximum number of pending reset streams allowed before a GOAWAY will be sent.
    ///
    /// This will default to whatever the default in h2 is. As of v0.3.17, it is 20.
    ///
    /// See <https://github.com/hyperium/hyper/issues/2877> for more information.
    pub(crate) http2_max_pending_accept_reset_streams: Option<usize>,

    /// Set whether TCP keepalive messages are enabled on accepted connections.
    ///
    /// If `None` is specified, keepalive is disabled, otherwise the duration
    /// specified will be the time to remain idle before sending TCP keepalive
    /// probes.
    ///
    /// Default is no keepalive (`None`)
    pub(crate) tcp_keepalive: Option<Duration>,

    /// Set the value of `TCP_NODELAY` option for accepted connections. Enabled by default.
    #[serde(default)]
    pub(crate) tcp_nodelay: bool,

    /// Sets the max size of received header frames.
    ///
    /// This will default to whatever the default in hyper is. As of v1.4.1, it is 16 KiB.
    pub(crate) http2_max_header_list_size: Option<u32>,

    /// Sets the maximum frame size to use for HTTP2.
    ///
    /// Passing `None` will do nothing.
    ///
    /// If not set, will default from underlying transport.
    pub(crate) max_frame_size: Option<u32>,

    /// Allow this server to accept http1 requests.
    ///
    /// Accepting http1 requests is only useful when developing `grpc-web`
    /// enabled services. If this setting is set to `true` but services are
    /// not correctly configured to handle grpc-web requests, your server may
    /// return confusing (but correct) protocol errors.
    ///
    /// Default is `false`.
    #[serde(default)]
    pub(crate) accept_http1: bool,

    #[serde(default)]
    pub(crate) graceful: bool,
}

fn default_binding() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
}

fn default_port() -> u16 {
    9090
}
