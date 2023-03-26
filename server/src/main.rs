use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;

use crate::api::setup_api_v1;
use crate::config::{
    AuthustConfiguration, InternalAuthustConfiguration, ListenConfiguration, PostgresConfiguration,
};
use crate::interface::setup_interface_router;
use crate::otel_middleware::{otel_layer, ExtensionLayer};
use crate::service::user::UserService;
use api::AuthServiceData;

use axum::error_handling::HandleErrorLayer;
use axum::routing::get;
use axum::{BoxError, Router};
use deadpool_postgres::{Manager, ManagerConfig, Pool};

use executor::FlowExecutor;
use futures::{Future, FutureExt};
use http::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey};

use once_cell::sync::Lazy;
use opentelemetry::sdk::resource::{EnvResourceDetector, ResourceDetector};
use opentelemetry::{sdk::Resource, Key, KeyValue};
use opentelemetry_otlp::{self as otlp, ExportConfig};

use otlp::{SpanExporterBuilder, TonicExporterBuilder, WithExportConfig};
use service::policy::PolicyService;

use storage::Storage;
use tokio::signal;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tracing::info;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{prelude::*, EnvFilter};

pub mod api;
pub mod auth;
pub mod config;
pub mod executor;
pub mod interface;
mod otel_middleware;
pub mod service;
mod state;
pub use state::*;

pub static INTERFACE_BASE_URI: Lazy<&'static str> = Lazy::new(base_uri);

fn base_uri() -> &'static str {
    let env = std::env::var("INTERFACE_BASE_URI").ok();
    match env {
        Some(v) => {
            tracing::info!("Serving web interface from url '{}' ", v);
            Box::leak(v.into_boxed_str())
        }
        None => {
            tracing::info!("Serving web interface from files");
            ""
        }
    }
}

#[tokio::main]
async fn main() {
    setup().await;
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/");
}

fn setup_grpc_exporter() -> TonicExporterBuilder {
    otlp::new_exporter().tonic().with_env()
}

#[cfg_attr(not(feature = "otlp-http-proto"), allow(unused))]
fn setup_span_exporter(config: &ExportConfig) -> SpanExporterBuilder {
    #[cfg(not(feature = "otlp-http-proto"))]
    return setup_grpc_exporter().into();

    #[cfg(feature = "otlp-http-proto")]
    return match config.protocol {
        opentelemetry_otlp::Protocol::Grpc => setup_grpc_exporter().into(),
        opentelemetry_otlp::Protocol::HttpBinary => otlp::new_exporter().http().with_env().into(),
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
                        .unwrap_or_else(|| env!("CARGO_BIN_NAME").to_string())
                }),
        )])
    }
}

async fn setup() {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "hyper=info,info");
    }
    let mut configuration =
        InternalAuthustConfiguration::load().expect("Failed to load configuration");
    let password = std::mem::take(&mut configuration.postgres.password);

    let config = ExportConfig::default();
    let resource = Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(ServiceNameDetector),
            Box::<EnvResourceDetector>::default(),
        ],
    );
    let tracer = otlp::new_pipeline()
        .tracing()
        .with_exporter(setup_span_exporter(&config))
        .with_trace_config(opentelemetry::sdk::trace::config().with_resource(resource))
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Failed to install opentelemetry tracer");
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
    info!("Setting up database...");

    // Initialize INTERFACE_BASE_URI
    #[allow(unused_must_use)]
    {
        INTERFACE_BASE_URI.len();
    }
    let pool = setup_database(&configuration.postgres, &password).await;
    let listen = configuration.listen.clone();
    let state = create_state(configuration, pool).await;
    let router = router(state).await;

    let shutdown_future = signal::ctrl_c().map(|fut| {
        fut.expect("Error occurred while waiting for Ctrl+C signal");
        tracing::info!("Received shutdown signal");
    });

    let app_future = start_server(listen, router, shutdown_future);
    app_future.await;
    tracing::info!("Shutting down opentelemetry provider");
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("Shutdown complete");
}

async fn setup_database(configuration: &PostgresConfiguration, password: &str) -> Pool {
    let mut cfg = tokio_postgres::Config::new();
    cfg.host(&configuration.host)
        .port(configuration.port)
        .dbname(&configuration.database)
        .user(&configuration.user)
        .password(password)
        .application_name("Authust");
    let mgr_config = ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    };
    let manager = Manager::from_config(cfg, tokio_postgres::NoTls, mgr_config);
    let pool = Pool::builder(manager).max_size(16).build().unwrap();
    let mut client = pool.get().await.expect("Failed to get connection!");
    embedded::migrations::runner()
        .run_async(client.as_mut().deref_mut())
        .await
        .expect("Failed to run migrations");
    info!("Running migrations on database...");
    info!("Database setup complete");
    pool
}

#[derive(Debug, Clone)]
pub struct AuthustState {
    pub configuration: AuthustConfiguration,
}
async fn create_state(config: InternalAuthustConfiguration, pool: Pool) -> SharedState {
    let storage = Storage::new(pool.clone()).await.unwrap();
    let storage_old = storage::create_manager(pool.clone());
    let policies = PolicyService::new(storage.clone());
    let defaults = Defaults::new(&storage, pool.clone()).await;
    let executor = FlowExecutor::new(Arc::new(storage.clone()), policies.clone());
    let users = UserService::new();
    let internal_state = InternalSharedState {
        users,
        executor,
        auth_data: AuthServiceData {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
        },
        storage,
        storage_old: storage_old.clone(),
        defaults: Arc::new(defaults),
        policies,
    };

    SharedState(Arc::new(internal_state))
}

async fn router(state: SharedState) -> Router<()> {
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let cors = tower_http::cors::CorsLayer::very_permissive();
    let service = ServiceBuilder::new()
        .layer(ExtensionLayer)
        .layer(otel_layer())
        .layer(CookieManagerLayer::new())
        .layer(HandleErrorLayer::new(handle_timeout_error))
        .timeout(Duration::from_secs(1));
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let service = service.layer(cors);

    Router::new()
        .route("/test", get(hello_world))
        .nest("/api/v1", setup_api_v1(state.clone()).await)
        .nest("/", setup_interface_router())
        .layer(service)
        .with_state(state)
}

async fn start_server(
    listen: ListenConfiguration,
    router: Router<()>,
    future: impl Future<Output = ()>,
) {
    let bind = axum::Server::bind(&listen.http);
    info!("Listening on {}...", listen.http);
    bind.serve(router.into_make_service())
        .with_graceful_shutdown(future)
        .await
        .expect("Server crashed");
    // server.await.expect("Server crashed");
}

async fn handle_timeout_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", err),
        )
    }
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}
