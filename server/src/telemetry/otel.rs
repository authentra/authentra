use std::{env, time::Duration};

use opentelemetry::{
    sdk::{
        resource::{EnvResourceDetector, ResourceDetector, TelemetryResourceDetector},
        trace::Tracer,
        Resource,
    },
    Key, KeyValue,
};
use opentelemetry_otlp::{
    ExportConfig, HasExportConfig, SpanExporterBuilder, TonicExporterBuilder, WithExportConfig,
};

struct DummyConfig(ExportConfig);

impl HasExportConfig for DummyConfig {
    fn export_config(&mut self) -> &mut ExportConfig {
        &mut self.0
    }
}

fn export_config() -> ExportConfig {
    DummyConfig(ExportConfig::default()).with_env().0
}

fn clone_config(config: &ExportConfig) -> ExportConfig {
    ExportConfig {
        endpoint: config.endpoint.clone(),
        protocol: config.protocol,
        timeout: config.timeout,
    }
}

fn setup_grpc_exporter(config: ExportConfig) -> TonicExporterBuilder {
    opentelemetry_otlp::new_exporter()
        .tonic()
        .with_export_config(config)
}

fn setup_span_exporter(config: &ExportConfig) -> SpanExporterBuilder {
    #[cfg(not(feature = "otlp-http-proto"))]
    return setup_grpc_exporter(clone_config(&config)).into();

    #[cfg(feature = "otlp-http-proto")]
    return match config.protocol {
        opentelemetry_otlp::Protocol::Grpc => setup_grpc_exporter(clone_config(config)).into(),
        opentelemetry_otlp::Protocol::HttpBinary => opentelemetry_otlp::new_exporter()
            .http()
            .with_export_config(clone_config(config))
            .into(),
    };
}

#[derive(Debug)]
pub struct SdkProvidedResourceDetector;

impl ResourceDetector for SdkProvidedResourceDetector {
    fn detect(&self, _timeout: Duration) -> Resource {
        Resource::new(vec![KeyValue::new(
            "service.name",
            env::var("OTEL_SERVICE_NAME")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    EnvResourceDetector::new()
                        .detect(Duration::from_secs(0))
                        .get(Key::new("service.name"))
                        .map(|v| v.to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| match option_env!("CARGO_BIN_NAME") {
                            Some(s) => s.to_string(),
                            None => "unknown_service".to_string(),
                        })
                }),
        )])
    }
}

fn resource() -> Resource {
    Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(SdkProvidedResourceDetector),
            Box::new(EnvResourceDetector::new()),
            Box::new(TelemetryResourceDetector),
        ],
    )
}

pub fn setup_otlp_tracer() -> Tracer {
    let config = export_config();
    let resource = resource();
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(setup_span_exporter(&config))
        .with_trace_config(opentelemetry::sdk::trace::config().with_resource(resource))
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Failed to install opentelemetry tracer")
}
