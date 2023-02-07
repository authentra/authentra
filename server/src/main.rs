#![feature(error_generic_member_access, provide_any)]

use std::path::Path;
use std::time::Duration;

use crate::api::setup_api_v1;
use crate::config::PostgresConfiguration;
use crate::config::{AuthustConfiguration, InternalAuthustConfiguration};
use crate::executor::FlowExecutor;
use crate::flow_storage::FlowStorage;
use api::AuthServiceData;

use axum::extract::FromRef;
use axum::routing::get;
use axum::{Extension, Router};
use handlebars::Handlebars;

use jsonwebtoken::{DecodingKey, EncodingKey};
use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions, PgPool};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

pub mod api;
pub mod auth;
pub mod config;
pub mod executor;
pub mod flow_storage;
pub mod model;

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
    let layer = tracing_subscriber::fmt::Layer::new();
    let registry = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(ErrorLayer::default())
        .with(layer);
    tracing::subscriber::set_global_default(registry).unwrap();
    info!("Setting up database...");
    let pool = setup_database(&configuration.postgres, &password)
        .await
        .expect("Failed to connect to database");
    info!("Running migrations on database...");
    MIGRATOR
        .run(&pool)
        .await
        .expect("Failed to run migrations on database");
    info!("Database setup complete");
    start_server(configuration, pool).await;
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
    PgPool::connect_with(options).await
}

#[derive(Debug, Clone)]
pub struct AuthustState {
    pub configuration: AuthustConfiguration,
}

fn setup_handlebars<'reg>() -> Handlebars<'reg> {
    let mut handlebars = Handlebars::new();
    fn register(reg: &mut Handlebars, name: &str, path: impl AsRef<Path>) {
        reg.register_template_file(name, path)
            .expect("Failed to register template");
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

async fn start_server(config: InternalAuthustConfiguration, pool: PgPool) {
    let storage = FlowStorage::new(pool.clone());
    let executor = FlowExecutor::new(storage);
    let _handlebars = setup_handlebars();
    let state = AppState {
        auth_data: AuthServiceData {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
        },
    };
    let router = Router::new()
        .route("/test", get(hello_world))
        .nest(
            "/api/v1",
            setup_api_v1(&config.secret, state.clone(), pool).await,
        )
        .layer(Extension(executor))
        .layer(Extension(state.auth_data.clone()))
        .layer(TraceLayer::new_for_http());
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
