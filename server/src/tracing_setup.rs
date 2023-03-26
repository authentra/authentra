use std::time::Duration;

use opentelemetry::{
    sdk::{
        resource::{EnvResourceDetector, ResourceDetector},
        trace::Tracer,
        Resource,
    },
    Key, KeyValue,
};
use opentelemetry_otlp::{
    ExportConfig, SpanExporterBuilder, TonicExporterBuilder, WithExportConfig,
};
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{prelude::*, EnvFilter};

fn setup_grpc_exporter() -> TonicExporterBuilder {
    opentelemetry_otlp::new_exporter().tonic().with_env()
}

#[cfg_attr(not(feature = "otlp-http-proto"), allow(unused))]
fn setup_span_exporter(config: &ExportConfig) -> SpanExporterBuilder {
    #[cfg(not(feature = "otlp-http-proto"))]
    return setup_grpc_exporter().into();

    #[cfg(feature = "otlp-http-proto")]
    return match config.protocol {
        opentelemetry_otlp::Protocol::Grpc => setup_grpc_exporter().into(),
        opentelemetry_otlp::Protocol::HttpBinary => {
            opentelemetry_otlp::new_exporter().http().with_env().into()
        }
    };
}

struct ServiceNameDetector;

impl ResourceDetector for ServiceNameDetector {
    fn detect(&self, _timeout: Duration) -> Resource {
        Resource::new(vec![KeyValue::new(
            "service.name",
            std::env::var("OTEL_SERVICE_NAME")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    EnvResourceDetector::new()
                        .detect(Duration::from_secs(0))
                        .get(Key::new("service.name"))
                        .map(|v| v.to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "authust_server".to_string())
                }),
        )])
    }
}

fn setup_otlp_tracer() -> Tracer {
    let config = ExportConfig::default();
    let resource = Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(ServiceNameDetector),
            Box::<EnvResourceDetector>::default(),
        ],
    );

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(setup_span_exporter(&config))
        .with_trace_config(opentelemetry::sdk::trace::config().with_resource(resource))
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Failed to install opentelemetry tracer")
}

pub fn setup_tracing() {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "hyper=info,info");
    }
    let tracer = setup_otlp_tracer();
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let layer = tracing_subscriber::fmt::Layer::new().with_filter(EnvFilter::from_default_env());
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::new(
            "tokio=trace,runtime=trace,trace,hyper=info,h2=info",
        ))
        .with(opentelemetry)
        .with(ErrorLayer::default())
        .with(layer);
    tracing::subscriber::set_global_default(registry).unwrap();
    LogTracer::init().unwrap();
}
