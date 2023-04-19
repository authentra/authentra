pub mod middleware;
mod otel;

pub use otel::setup_otlp_tracer;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

pub fn setup_tracing() {
    let tracer = setup_otlp_tracer();
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let filter = match std::env::var("RUST_LOG") {
        Ok(v) => v,
        Err(err) => match err {
            std::env::VarError::NotPresent => "info".into(),
            std::env::VarError::NotUnicode(err) => panic!("RUST_LOG is not unicode! {err:?}"),
        },
    };
    let filter = EnvFilter::try_new(filter).unwrap();
    let layer = tracing_subscriber::fmt::Layer::new().with_filter(filter);
    let registry = tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(opentelemetry)
        .with(layer);
    tracing::subscriber::set_global_default(registry).unwrap();
}
