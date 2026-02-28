//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-opendal)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]
pub mod config;

use crate::config::*;
use anyhow::Result;
use opendal::Operator;
use summer::app::AppBuilder;
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::plugin::{MutableComponentRegistry, Plugin};
use std::str::FromStr;

pub type Op = Operator;

pub struct OpenDALPlugin;

#[async_trait]
impl Plugin for OpenDALPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<OpenDALConfig>()
            .expect("OpenDAL plugin config load failed");

        let connect = Self::operator(config).expect("OpenDAL operator construct failed");
        app.add_component(connect);
    }
}

impl OpenDALPlugin {
    pub fn operator(config: OpenDALConfig) -> Result<Operator> {
        let scheme = opendal::Scheme::from_str(&config.scheme).map_err(|err| {
            opendal::Error::new(opendal::ErrorKind::Unexpected, "not supported scheme")
                .set_source(err)
        })?;

        let options = config.options.unwrap_or_default();

        #[allow(unused_mut)]
        let mut op = Operator::via_iter(scheme, options)?;

        if let Some(layers) = config.layers {
            if let Some(layer) = layers.into_iter().next() {
                log::debug!("layer-{layer} enable");
                match layer {
                    #[cfg(feature = "layers-chaos")]
                    Layers::Chaos { error_ratio } => {
                        op = op.layer(opendal::layers::ChaosLayer::new(error_ratio));
                    }
                    #[cfg(feature = "layers-metrics")]
                    Layers::Metrics => {
                        op = op.layer(opendal::layers::MetricsLayer::default());
                    }
                    #[cfg(feature = "layers-mime-guess")]
                    Layers::MimeGuess => {
                        op = op.layer(opendal::layers::MimeGuessLayer::default());
                    }
                    #[cfg(feature = "layers-prometheus")]
                    Layers::Prometheus {
                        duration_seconds_buckets,
                        bytes_buckets,
                    } => {
                        let mut builder = opendal::layers::PrometheusLayer::builder();
                        if let Some(duration_seconds_buckets) = duration_seconds_buckets {
                            builder = builder.duration_seconds_buckets(duration_seconds_buckets);
                        }
                        if let Some(bytes_buckets) = bytes_buckets {
                            builder = builder.bytes_buckets(bytes_buckets);
                        }
                        let prometheus_layer = builder
                            .register_default()
                            .expect("Failed to register with the global registry");

                        op = op.layer(prometheus_layer);
                    }
                    #[cfg(feature = "layers-prometheus-client")]
                    Layers::PrometheusClient => {
                        let mut registry = prometheus_client::registry::Registry::default();
                        op = op.layer(
                            opendal::layers::PrometheusClientLayer::builder()
                                .register(&mut registry),
                        );
                    }
                    #[cfg(feature = "layers-fastrace")]
                    Layers::Fastrace => {
                        op = op.layer(opendal::layers::FastraceLayer);
                    }
                    #[cfg(feature = "layers-tracing")]
                    Layers::Tracing => {
                        op = op.layer(opendal::layers::TracingLayer);
                    }
                    #[cfg(feature = "layers-otel-trace")]
                    Layers::OtelTrace => {
                        op = op.layer(opendal::layers::OtelTraceLayer);
                    }
                    #[cfg(feature = "layers-throttle")]
                    Layers::Throttle { bandwidth, burst } => {
                        op = op.layer(opendal::layers::ThrottleLayer::new(bandwidth, burst));
                    }
                    #[cfg(feature = "layers-await-tree")]
                    Layers::AwaitTree => {
                        op = op.layer(opendal::layers::AwaitTreeLayer::new());
                    }
                    #[cfg(feature = "layers-async-backtrace")]
                    Layers::AsyncBacktrace => {
                        op = op.layer(opendal::layers::AsyncBacktraceLayer);
                    }
                    #[cfg(feature = "layers-dtrace")]
                    Layers::Dtrace => {
                        #[cfg(target_os = "linux")]
                        {
                            op = op.layer(opendal::layers::DtraceLayer::default());
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            log::warn!("DtraceLayer is only supported on Linux, skipping");
                        }
                    }
                    #[allow(unreachable_patterns)]
                    _ => {
                        panic!("Maybe you forgotten to enable the [services-{layer}] feature!");
                    }
                }
            }
        }

        Ok(op)
    }
}

#[cfg(test)]
mod tests {
    use super::config::*;
    use super::OpenDALPlugin;
    use log::debug;

    #[tokio::test]
    async fn blocking() {
        let config = OpenDALConfig {
            scheme: "memory".to_string(),
            options: None,
            layers: None,
        };

        debug!("config: {config:?}");

        let op = OpenDALPlugin::operator(config).unwrap();
        let o = op.write("test", b"test".to_vec()).await;
        assert!(o.is_ok(), "Write operation failed: {o:?}");

        let res = op.read("test").await.unwrap();

        assert_eq!(res.to_vec(), b"test".to_vec());
    }
}
