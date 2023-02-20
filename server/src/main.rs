use std::future::ready;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::api::setup_api_v1;
use crate::api::sql_tx::TxLayer;
use crate::config::PostgresConfiguration;
use crate::config::{AuthustConfiguration, InternalAuthustConfiguration};
use crate::executor::FlowExecutor;
use crate::flow_storage::{FlowStorage, FreezedStorage, ReferenceLookup, ReverseLookup};
use crate::interface::setup_interface_router;
use crate::service::user::UserService;
use api::AuthServiceData;

use axum::error_handling::HandleErrorLayer;
use axum::extract::{FromRef, MatchedPath};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{BoxError, Router};
use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, PoolError};
use handlebars::Handlebars;

use http::{Request, StatusCode};
use jsonwebtoken::{DecodingKey, EncodingKey};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
use model::{Flow, Policy, Reference, Tenant};
use service::policy::PolicyService;
use sqlx::query;
use sqlx::{postgres::PgConnectOptions, ConnectOptions, PgPool};
use tokio::time::Instant;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
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
pub mod service;

#[tokio::main]
async fn main() {
    setup().await;
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/");
}

async fn setup() {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "hyper=info,info");
    }
    let mut configuration =
        InternalAuthustConfiguration::load().expect("Failed to load configuration");
    let password = std::mem::replace(&mut configuration.postgres.password, String::new());

    let opentelemetry = if let Some(endpoint) = &configuration.jaeger_endpoint {
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_endpoint(endpoint)
            .with_service_name("authust")
            .install_simple()
            .expect("Faield to install opentelemetry jaeger tracer");
        let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        println!("Enabling OTEL Jaeger");
        Some(opentelemetry)
    } else {
        None
    };
    let layer = tracing_subscriber::fmt::Layer::new().with_filter(EnvFilter::from_default_env());
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::new(
            "tokio=trace,runtime=trace,trace,hyper=trace",
        ))
        .with(opentelemetry)
        .with(ErrorLayer::default())
        .with(layer);
    tracing::subscriber::set_global_default(registry).unwrap();
    LogTracer::init().unwrap();
    info!("Setting up database...");
    let (sqlx_pool, pool) = setup_database(&configuration.postgres, &password)
        .await
        .expect("Failed to connect to database");
    let metrics_future = start_metrics_server(configuration.listen.metrics);
    let app_future = start_server(configuration, sqlx_pool, pool);
    tokio::join!(metrics_future, app_future);
    opentelemetry::global::shutdown_tracer_provider();
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

fn setup_handlebars<'reg>() -> Handlebars<'reg> {
    let mut handlebars = Handlebars::new();
    fn register(_reg: &mut Handlebars, _name: &str, _path: impl AsRef<Path>) {
        // reg.register_template_file(name, path)
        //     .expect("Failed to register template");
    }
    register(&mut handlebars, "flow", "templates/flow.hbs");
    handlebars
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

async fn start_server(config: InternalAuthustConfiguration, sqlx_pool: PgPool, pool: Pool) {
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
    let _handlebars = setup_handlebars();
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
        .layer(middleware::from_fn(track_metrics))
        .layer(TraceLayer::new_for_http())
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

async fn start_metrics_server(addr: SocketAddr) {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    let recorder = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .unwrap()
        .install_recorder()
        .unwrap();
    let app = Router::new().route("/metrics", get(move || ready(recorder.render())));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap()
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}

async fn track_metrics<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone();

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status),
    ];

    metrics::increment_counter!("http_requests_total", &labels);
    metrics::histogram!("http_requests_duration_seconds", latency, &labels);

    response
}
