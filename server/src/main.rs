use std::ops::DerefMut;

use std::sync::Arc;
use std::time::Duration;

use crate::api::setup_api_v1;
use crate::api::sql_tx::TxLayer;
use crate::config::PostgresConfiguration;
use crate::config::{AuthustConfiguration, InternalAuthustConfiguration};
use crate::flow_storage::{FlowStorage, FreezedStorage, ReferenceLookup, ReverseLookup};
use crate::interface::setup_interface_router;
use crate::otel_middleware::{otel_layer, ExtensionLayer};
use crate::service::user::UserService;
use api::AuthServiceData;

use axum::error_handling::HandleErrorLayer;
use axum::extract::FromRef;
use axum::routing::get;
use axum::{BoxError, Router};
use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, PoolError};

use executor::FlowExecutor;
use futures::{Future, FutureExt};
use http::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey};
use model::{Flow, Policy, Reference, Tenant};

use opentelemetry::sdk::resource::{EnvResourceDetector, ResourceDetector};
use opentelemetry::{sdk::Resource, Key, KeyValue};
use opentelemetry_otlp::{self as otlp, ExportConfig};

use otlp::{SpanExporterBuilder, TonicExporterBuilder, WithExportConfig};
use service::policy::PolicyService;
use sqlx::query;
use sqlx::{postgres::PgConnectOptions, ConnectOptions, PgPool};
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
pub mod flow_storage;
pub mod interface;
mod otel_middleware;
pub mod service;

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
    let password = std::mem::replace(&mut configuration.postgres.password, String::new());

    let config = ExportConfig::default();
    let resource = Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(ServiceNameDetector),
            Box::new(EnvResourceDetector::default()),
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
    let shutdown_future = signal::ctrl_c().map(|fut| {
        fut.expect("Error occurred while waiting for Ctrl+C signal");
        tracing::info!("Received shutdown signal");
    });
    let (sqlx_pool, pool) = setup_database(&configuration.postgres, &password)
        .await
        .expect("Failed to connect to database");
    let app_future = start_server(configuration, sqlx_pool, pool, shutdown_future);
    app_future.await;
    tracing::info!("Shutting down opentelemetry provider");
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("Shutdown complete");
}

async fn setup_database(
    configuration: &PostgresConfiguration,
    password: &str,
) -> Result<(PgPool, Pool), sqlx::Error> {
    let mut options = PgConnectOptions::new_without_pgpass()
        .host(&configuration.host)
        .port(configuration.port)
        .database(&configuration.database)
        .username(&configuration.user)
        .password(password)
        .application_name("Authust");
    options.log_statements(tracing::log::LevelFilter::Debug);
    // options.log_slow_statements(tracing::log::LevelFilter::Warn, Duration::from_millis(100));
    let sqlx_pool = PgPool::connect_with(options).await?;
    let mut cfg = tokio_postgres::Config::new();
    cfg.host(&configuration.host)
        .port(configuration.port)
        .dbname(&configuration.database)
        .user(&configuration.user)
        .password(&password)
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
    Ok((sqlx_pool, pool))
}

#[derive(Debug, Clone)]
pub struct AuthustState {
    pub configuration: AuthustConfiguration,
}

#[derive(Clone)]
pub struct AppState {
    auth_data: AuthServiceData,
}

impl FromRef<AppState> for AuthServiceData {
    fn from_ref(input: &AppState) -> Self {
        input.auth_data.clone()
    }
}

async fn preload(pool: &PgPool, storage: &FlowStorage) -> Result<(), sqlx::Error> {
    info!("Preloading flows...");
    let ids = query!("select uid from flows")
        .fetch_all(pool)
        .await
        .expect("Failed to load flow ids")
        .into_iter()
        .map(|rec| Reference::<Flow>::new_uid(rec.uid));
    let freezed = FreezedStorage::new(storage.clone());
    for reference in ids {
        let flow = freezed
            .lookup_reference(&reference)
            .await
            .expect("Failed to preload flow");
        flow.reverse_lookup(&freezed).await;
    }
    info!("Preloading tenants...");
    let tenants: Vec<Reference<Tenant>> = query!("select uid from tenants")
        .fetch_all(pool)
        .await
        .expect("Failed to load tenant ids")
        .into_iter()
        .map(|rec| Reference::new_uid(rec.uid))
        .collect();
    for reference in tenants {
        storage.lookup_reference(&reference).await;
    }
    info!("Preloading policies...");
    let policies: Vec<Reference<Policy>> = query!("select uid from policies")
        .fetch_all(pool)
        .await
        .expect("Failed to load policy ids")
        .into_iter()
        .map(|rec| Reference::new_uid(rec.uid))
        .collect();
    for policy in policies {
        storage.lookup_reference(&policy).await;
    }

    info!("Preload complete");
    Ok(())
}

#[derive(Clone)]
pub struct SharedState(Arc<InternalSharedState>, PgPool);

impl SharedState {
    pub fn users(&self) -> &UserService {
        &self.0.users
    }

    pub fn executor(&self) -> &FlowExecutor {
        &self.0.executor
    }

    pub fn auth_data(&self) -> &AuthServiceData {
        &self.0.auth_data
    }

    pub fn storage(&self) -> &FlowStorage {
        &self.0.storage
    }
    pub fn defaults(&self) -> &Defaults {
        &self.0.defaults.as_ref()
    }
    pub fn policies(&self) -> &PolicyService {
        &self.0.policies
    }
}

impl FromRef<SharedState> for PgPool {
    fn from_ref(input: &SharedState) -> Self {
        input.1.clone()
    }
}

struct InternalSharedState {
    users: UserService,
    executor: FlowExecutor,
    auth_data: AuthServiceData,
    storage: FlowStorage,
    defaults: Arc<Defaults>,
    policies: PolicyService,
}

pub struct Defaults {
    storage: FlowStorage,
    sqlx_pool: PgPool,
    pool: Pool,
    tenant: Option<Arc<Tenant>>,
}

impl Defaults {
    pub fn tenant(&self) -> Option<Arc<Tenant>> {
        self.tenant.clone()
    }

    pub fn sqlx_pool(&self) -> &PgPool {
        &self.sqlx_pool
    }
    pub fn pool(&self) -> Pool {
        self.pool.clone()
    }
    pub async fn connection(&self) -> Result<Object, PoolError> {
        self.pool.get().await
    }
}

impl Defaults {
    pub async fn new(storage: FlowStorage, sqlx_pool: PgPool, pool: Pool) -> Self {
        let default = Self::find_default(&storage, &sqlx_pool).await;
        Defaults {
            storage,
            sqlx_pool,
            pool,
            tenant: default,
        }
    }

    async fn find_default(storage: &FlowStorage, pool: &PgPool) -> Option<Arc<Tenant>> {
        let res = match query!("select uid from tenants where is_default = true")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
        {
            Some(res) => res,
            None => return None,
        };
        let reference: Reference<Tenant> = Reference::new_uid(res.uid);
        let tenant = storage.lookup_reference(&reference).await;
        tenant
    }
}

async fn start_server(
    config: InternalAuthustConfiguration,
    sqlx_pool: PgPool,
    pool: Pool,
    future: impl Future<Output = ()>,
) {
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let cors = tower_http::cors::CorsLayer::very_permissive();
    let storage = FlowStorage::new(sqlx_pool.clone(), pool.clone());
    preload(&sqlx_pool, &storage)
        .await
        .expect("Preloading failed");
    let policies = PolicyService::new(storage.clone(), pool.clone());
    let defaults = Defaults::new(storage.clone(), sqlx_pool.clone(), pool.clone()).await;
    let executor = FlowExecutor::new(storage.clone(), policies.clone());
    let users = UserService::new();
    let internal_state = InternalSharedState {
        users,
        executor,
        auth_data: AuthServiceData {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
        },
        storage: storage.clone(),
        defaults: Arc::new(defaults),
        policies,
    };
    let state = SharedState(Arc::new(internal_state), sqlx_pool.clone());
    let service = ServiceBuilder::new()
        .layer(ExtensionLayer)
        .layer(otel_layer())
        .layer(CookieManagerLayer::new())
        .layer(TxLayer::new(sqlx_pool))
        .layer(HandleErrorLayer::new(handle_timeout_error))
        .timeout(Duration::from_secs(1));
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let service = service.layer(cors);
    let router = Router::new()
        .route("/test", get(hello_world))
        .nest("/api/v1", setup_api_v1(&config.secret, state.clone()).await)
        .nest("/", setup_interface_router())
        .layer(service)
        .with_state(state);
    let bind = axum::Server::bind(&config.listen.http);
    info!("Listening on {}...", config.listen.http);
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
