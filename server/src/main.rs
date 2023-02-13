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

use axum::extract::FromRef;
use axum::routing::get;
use axum::Router;
use handlebars::Handlebars;

use jsonwebtoken::{DecodingKey, EncodingKey};
use model::{Flow, Reference, Tenant};
use sqlx::query;
use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions, PgPool};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_error::ErrorLayer;
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

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

async fn setup() {
    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "hyper=info,debug");
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
        Some(opentelemetry)
    } else {
        None
    };
    let layer = tracing_subscriber::fmt::Layer::new().with_filter(EnvFilter::from_default_env());
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::new("tokio=trace,runtime=trace,trace"))
        .with(opentelemetry)
        .with(ErrorLayer::default())
        .with(layer);
    tracing::subscriber::set_global_default(registry).unwrap();
    info!("Setting up database...");
    let pool = setup_database(&configuration.postgres, &password)
        .await
        .expect("Failed to connect to database");
    start_server(configuration, pool).await;
    opentelemetry::global::shutdown_tracer_provider();
}

async fn setup_database(
    configuration: &PostgresConfiguration,
    password: &str,
) -> Result<PgPool, sqlx::Error> {
    let mut options = PgConnectOptions::new_without_pgpass()
        .host(&configuration.host)
        .port(configuration.port)
        .database(&configuration.database)
        .username(&configuration.user)
        .password(password)
        .application_name("Authust");
    options.log_statements(tracing::log::LevelFilter::Off);
    options.log_slow_statements(tracing::log::LevelFilter::Warn, Duration::from_millis(100));
    let pool = PgPool::connect_with(options).await?;
    info!("Running migrations on database...");
    MIGRATOR
        .run(&pool)
        .await
        .expect("Failed to run migrations on database");
    info!("Database setup complete");
    Ok(pool)
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
    // let tenants = query!("select uid from tenants")
    //     .fetch_all(pool)
    //     .await
    //     .expect("Failed to load tenant ids")
    //     .into_iter()
    //     .map(|rec| Reference::<Tenant>::new_uid(rec.uid));
    info!("Preload complete");
    Ok(())
}

#[derive(Clone)]
pub struct SharedState(Arc<InternalSharedState>);

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
}

struct InternalSharedState {
    users: UserService,
    executor: FlowExecutor,
    auth_data: AuthServiceData,
    storage: FlowStorage,
    defaults: Arc<Defaults>,
}

pub struct Defaults {
    storage: FlowStorage,
    pool: PgPool,
    tenant: Option<Arc<Tenant>>,
}

impl Defaults {
    pub fn tenant(&self) -> Option<Arc<Tenant>> {
        self.tenant.clone()
    }
}

impl Defaults {
    pub async fn new(storage: FlowStorage, pool: PgPool) -> Self {
        let default = Self::find_default(&storage, &pool).await;
        Defaults {
            storage,
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

async fn start_server(config: InternalAuthustConfiguration, pool: PgPool) {
    #[cfg(debug_assertions)]
    #[cfg(feature = "dev-mode")]
    let cors = tower_http::cors::CorsLayer::very_permissive();
    let storage = FlowStorage::new(pool.clone());
    preload(&pool, &storage).await.expect("Preloading failed");
    let defaults = Defaults::new(storage.clone(), pool.clone()).await;
    let executor = FlowExecutor::new(storage.clone());
    let users = UserService::new();
    let _handlebars = setup_handlebars();
    let internal_state = InternalSharedState {
        users,
        executor,
        auth_data: AuthServiceData {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
        },
        storage,
        defaults: Arc::new(defaults),
    };
    let state = SharedState(Arc::new(internal_state));
    let service = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CookieManagerLayer::new())
        .layer(TxLayer::new(pool));
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

async fn hello_world() -> &'static str {
    "Hello, World!"
}
