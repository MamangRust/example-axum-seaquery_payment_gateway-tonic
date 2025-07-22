use std::sync::OnceLock;

use opentelemetry::{Context, global};
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource, logs::SdkLoggerProvider, metrics::SdkMeterProvider, trace::SdkTracerProvider,
};
use tokio::time::Instant;

#[derive(Clone)]
pub struct Telemetry {
    service_name: String,
}

pub struct TracingContext {
    pub cx: Context,
    pub start_time: Instant,
}

impl Telemetry {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    fn get_resource(&self) -> Resource {
        static RESOURCE: OnceLock<Resource> = OnceLock::new();
        RESOURCE
            .get_or_init(|| {
                Resource::builder()
                    .with_service_name(self.service_name.clone())
                    .build()
            })
            .clone()
    }

    pub fn init_tracer(&self) -> SdkTracerProvider {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint("http://otel-collector:4317")
            .build()
            .expect("Failed to create span exporter");

        let provider = SdkTracerProvider::builder()
            .with_resource(self.get_resource())
            .with_batch_exporter(exporter)
            .build();

        global::set_tracer_provider(provider.clone());

        provider
    }

    pub fn init_meter(&self) -> SdkMeterProvider {
        let exporter = MetricExporter::builder()
            .with_tonic()
            .with_endpoint("http://otel-collector:4317")
            .build()
            .expect("Failed to create metric exporter");

        let metrics = SdkMeterProvider::builder()
            .with_resource(self.get_resource())
            .with_periodic_exporter(exporter)
            .build();

        global::set_meter_provider(metrics.clone());

        metrics
    }

    pub fn init_logger(&self) -> SdkLoggerProvider {
        let exporter = LogExporter::builder()
            .with_tonic()
            .with_endpoint("http://otel-collector:4317")
            .build()
            .expect("Failed to create log exporter");

        SdkLoggerProvider::builder()
            .with_resource(self.get_resource())
            .with_batch_exporter(exporter)
            .build()
    }
}
