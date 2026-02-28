//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-opentelemetry)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod metrics;
pub mod trace;
pub mod util;

use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter};
#[rustfmt::skip]
pub use opentelemetry_otlp::{
    OTEL_EXPORTER_OTLP_COMPRESSION,
    OTEL_EXPORTER_OTLP_ENDPOINT,
    OTEL_EXPORTER_OTLP_HEADERS,
    OTEL_EXPORTER_OTLP_TIMEOUT,
    // logs
    OTEL_EXPORTER_OTLP_LOGS_COMPRESSION,
    OTEL_EXPORTER_OTLP_LOGS_ENDPOINT,
    OTEL_EXPORTER_OTLP_LOGS_HEADERS,
    OTEL_EXPORTER_OTLP_LOGS_TIMEOUT,
    // metrics
    OTEL_EXPORTER_OTLP_METRICS_COMPRESSION,
    OTEL_EXPORTER_OTLP_METRICS_ENDPOINT,
    OTEL_EXPORTER_OTLP_METRICS_HEADERS,
    OTEL_EXPORTER_OTLP_METRICS_TIMEOUT,
    // trace
    OTEL_EXPORTER_OTLP_TRACES_COMPRESSION,
    OTEL_EXPORTER_OTLP_TRACES_ENDPOINT,
    OTEL_EXPORTER_OTLP_TRACES_HEADERS,
    OTEL_EXPORTER_OTLP_TRACES_TIMEOUT,
};
pub use opentelemetry::{global, KeyValue};
pub use opentelemetry_sdk::Resource;
pub use opentelemetry_semantic_conventions::resource::*;

use anyhow::Context;
use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry::trace::TracerProvider;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_semantic_conventions::attribute;
use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::{ConfigRegistry, Configurable};
use summer::plugin::component::ComponentRef;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry};
use summer::{app::AppBuilder, error::Result, plugin::Plugin};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};

summer::submit_config_schema!("opentelemetry", OpenTelemetryConfig);

#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "opentelemetry"]
struct OpenTelemetryConfig {
    #[serde(default)]
    enable: bool,
}

/// Routers collection
pub type KeyValues = Vec<KeyValue>;

pub struct OpenTelemetryPlugin;

impl Plugin for OpenTelemetryPlugin {
    fn immediately_build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<OpenTelemetryConfig>()
            .expect("opentelemetry plugin config load failed");
        if !config.enable {
            return;
        }
        let resource = Self::build_resource(app);
        let log_provider = Self::init_logs(resource.clone());
        let meter_provider = Self::init_metrics(resource.clone());
        let tracer_provider = Self::init_tracer(resource);

        let tracer = tracer_provider.tracer(env!("CARGO_PKG_NAME"));

        let log_layer = OpenTelemetryTracingBridge::new(&log_provider);
        let metric_layer = MetricsLayer::new(meter_provider.clone());
        let trace_layer = OpenTelemetryLayer::new(tracer);

        app.add_layer(trace_layer)
            .add_layer(log_layer)
            .add_layer(metric_layer)
            .add_shutdown_hook(move |_| {
                Box::new(Self::shutdown(
                    tracer_provider,
                    meter_provider,
                    log_provider,
                ))
            });
    }

    fn immediately(&self) -> bool {
        true
    }
}

impl OpenTelemetryPlugin {
    fn init_logs(resource: Resource) -> SdkLoggerProvider {
        let exporter = {
            #[cfg(feature = "http")]
            {
                LogExporter::builder()
                    .with_http()
                    .build()
                    .expect("build http log exporter failed")
            }

            #[cfg(all(not(feature = "http"), feature = "grpc"))]
            {
                LogExporter::builder()
                    .with_tonic()
                    .build()
                    .expect("build grpc log exporter failed")
            }

            #[cfg(not(any(feature = "http", feature = "grpc")))]
            compile_error!(
                "You must enable either the 'http' or 'grpc' feature for the log exporter."
            );
        };
        SdkLoggerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build()
    }

    fn init_metrics(resource: Resource) -> SdkMeterProvider {
        let exporter = {
            #[cfg(feature = "http")]
            {
                MetricExporter::builder()
                    .with_http()
                    .build()
                    .expect("build http metric exporter failed")
            }

            #[cfg(all(not(feature = "http"), feature = "grpc"))]
            {
                MetricExporter::builder()
                    .with_tonic()
                    .build()
                    .expect("build grpc metric exporter failed")
            }

            #[cfg(not(any(feature = "http", feature = "grpc")))]
            compile_error!(
                "You must enable either the 'http' or 'grpc' feature for the log exporter."
            );
        };

        let provider = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_periodic_exporter(exporter)
            .build();

        global::set_meter_provider(provider.clone());
        tracing::debug!("metrics provider installed");

        provider
    }

    fn init_tracer(resource: Resource) -> SdkTracerProvider {
        let exporter = {
            #[cfg(feature = "http")]
            {
                SpanExporter::builder()
                    .with_http()
                    .build()
                    .expect("build http span exporter failed")
            }

            #[cfg(all(not(feature = "http"), feature = "grpc"))]
            {
                SpanExporter::builder()
                    .with_tonic()
                    .build()
                    .expect("build grpc span exporter failed")
            }

            #[cfg(not(any(feature = "http", feature = "grpc")))]
            compile_error!(
                "You must enable either the 'http' or 'grpc' feature for the log exporter."
            );
        };

        global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
            Box::new(BaggagePropagator::new()),
            Box::new(TraceContextPropagator::new()),
        ]));
        #[cfg(feature = "jaeger")]
        global::set_text_map_propagator(opentelemetry_jaeger_propagator::Propagator::new());
        #[cfg(feature = "zipkin")]
        global::set_text_map_propagator(opentelemetry_zipkin::Propagator::new());

        let provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build();

        global::set_tracer_provider(provider.clone());
        tracing::debug!("tracer provider installed");

        provider
    }

    fn build_resource(app: &AppBuilder) -> Resource {
        let mut key_values = app.get_component::<KeyValues>().unwrap_or_default();
        key_values.push(KeyValue::new(
            attribute::DEPLOYMENT_ENVIRONMENT_NAME,
            format!("{:?}", app.get_env()),
        ));
        let mut builder = Resource::builder();
        #[cfg(feature = "more-resource")]
        {
            builder = builder.with_detectors(&[
                Box::new(opentelemetry_resource_detectors::HostResourceDetector::default()),
                Box::new(opentelemetry_resource_detectors::OsResourceDetector),
                Box::new(opentelemetry_resource_detectors::ProcessResourceDetector),
            ]);
        }
        builder = builder.with_attributes(key_values);
        builder.build()
    }

    async fn shutdown(
        tracer_provider: SdkTracerProvider,
        meter_provider: SdkMeterProvider,
        log_provider: SdkLoggerProvider,
    ) -> Result<String> {
        tracer_provider
            .shutdown()
            .context("shutdown tracer provider failed")?;
        meter_provider
            .shutdown()
            .context("shutdown meter provider failed")?;
        log_provider
            .shutdown()
            .context("shutdown log provider failed")?;
        Ok("OpenTelemetry shutdown successful".into())
    }
}

pub trait ResourceConfigurator {
    fn opentelemetry_attrs<KV>(&mut self, kvs: KV) -> &mut Self
    where
        KV: IntoIterator<Item = KeyValue>;
}

impl ResourceConfigurator for AppBuilder {
    fn opentelemetry_attrs<KV>(&mut self, kvs: KV) -> &mut Self
    where
        KV: IntoIterator<Item = KeyValue>,
    {
        if let Some(key_values) = self.get_component_ref::<KeyValues>() {
            unsafe {
                let raw_ptr = ComponentRef::into_raw(key_values);
                let key_values = &mut *(raw_ptr as *mut KeyValues);
                key_values.extend(kvs);
            }
            self
        } else {
            let mut key_values: KeyValues = vec![];
            key_values.extend(kvs);
            self.add_component(key_values)
        }
    }
}
