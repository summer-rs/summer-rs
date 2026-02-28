use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

summer::submit_config_schema!("opendal", OpenDALConfig);

#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "opendal"]
pub struct OpenDALConfig {
    /// [Services](opendal::Scheme)  that OpenDAL supports
    ///
    /// According trait [opendal::Scheme::from_str] to set the config,
    /// visit [opendal](https://docs.rs/opendal/latest/opendal/services/index.html) to learn more
    ///
    /// **Required**: Need enable feature like `services-{$scheme}`
    pub scheme: String,

    /// Different options for different [scheme](https://docs.rs/opendal/latest/opendal/services/index.html),
    ///
    /// **Optional**
    pub options: Option<HashMap<String, String>>,

    /// OpenDAL provides a variety of [layers](https://docs.rs/opendal/latest/opendal/layers/index.html)
    ///
    /// **Optional**: Need enable feature
    pub layers: Option<Vec<Layers>>,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub enum Layers {
    /// [OpenDAL ChaosLayer](https://docs.rs/opendal/latest/opendal/layers/struct.ChaosLayer.html)
    ///
    /// Enable feature: `layers-chaos`
    Chaos { error_ratio: f64 },
    /// [OpenDAL MetricsLayer](https://docs.rs/opendal/latest/opendal/layers/struct.MetricsLayer.html)
    ///
    /// Enable feature: `layers-metrics`
    Metrics,
    /// [OpenDAL MimeGuessLayer](https://docs.rs/opendal/latest/opendal/layers/struct.MimeGuessLayer.html)
    ///
    /// Enable feature: `layers-mime-guess`
    MimeGuess,
    /// [OpenDAL PrometheusLayer](https://docs.rs/opendal/latest/opendal/layers/struct.PrometheusLayer.html)
    ///
    /// Enable feature: `layers-prometheus`
    Prometheus {
        duration_seconds_buckets: Option<Vec<f64>>,
        bytes_buckets: Option<Vec<f64>>,
    },
    /// [OpenDAL PrometheusClientLayer](https://docs.rs/opendal/latest/opendal/layers/struct.PrometheusClientLayer.html)
    ///
    /// Enable feature: `layers-prometheus-client`
    PrometheusClient,
    /// [OpenDAL FastraceLayer](https://docs.rs/opendal/latest/opendal/layers/struct.FastraceLayer.html)
    ///
    /// Enable feature: `layers-fastrace`
    Fastrace,
    /// [OpenDAL TracingLayer](https://docs.rs/opendal/latest/opendal/layers/struct.TracingLayer.html)
    ///
    /// Enable feature: `layers-tracing`
    Tracing,
    /// [OpenDAL OtelTraceLayer](https://docs.rs/opendal/latest/opendal/layers/struct.OtelTraceLayer.html)
    ///
    /// Enable feature: `layers-otel-trace`
    OtelTrace,
    /// [OpenDAL ThrottleLayer](https://docs.rs/opendal/latest/opendal/layers/struct.ThrottleLayer.html)
    ///
    /// Enable feature: `layers-throttle`
    Throttle { bandwidth: u32, burst: u32 },
    /// [OpenDAL AwaitTreeLayer](https://docs.rs/opendal/latest/opendal/layers/struct.AwaitTreeLayer.html)
    ///
    /// Enable feature: `layers-await-tree`
    AwaitTree,
    /// [OpenDAL AsyncBacktraceLayer](https://docs.rs/opendal/latest/opendal/layers/struct.AsyncBacktraceLayer.html)
    ///
    /// Enable feature: `layers-async-backtrace`
    AsyncBacktrace,
    /// [OpenDAL DtraceLayer](https://docs.rs/opendal/latest/opendal/layers/struct.DtraceLayer.html)
    ///
    /// On Linux and enable feature: `layers-dtrace`
    Dtrace,
}

impl Display for Layers {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let lf = match self {
            Layers::Chaos { .. } => "chaos",
            Layers::Metrics => "metrics",
            Layers::MimeGuess => "mimeGuess",
            Layers::Prometheus { .. } => "prometheus",
            Layers::PrometheusClient => "prometheus-client",
            Layers::Fastrace => "fastrace",
            Layers::Tracing => "tracing",
            Layers::OtelTrace => "otel-trace",
            Layers::Throttle { .. } => "throttle",
            Layers::AwaitTree => "awaitTree",
            Layers::AsyncBacktrace => "async-backtrace",
            Layers::Dtrace => "dtrace",
        };
        write!(f, "{lf}")
    }
}
