#![feature(error_generic_member_access, provide_any)]

use std::path::Path;
use std::time::Duration;

use crate::api::{setup_api_v1, CSRF_HEADER};
use crate::config::PostgresConfiguration;
use crate::config::{AuthustConfiguration, InternalAuthustConfiguration};
use crate::executor::FlowExecutor;
use crate::flow_storage::FlowStorage;
use crate::tracing_mw::Tracing;
use handlebars::Handlebars;
use http::header::AUTHORIZATION;
use poem::listener::TcpListener;
use poem::middleware::{AddData, SensitiveHeader};
use poem::{handler, EndpointExt, Route, Server};
use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions, PgPool};
use tracing::info;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

pub mod api;
pub mod auth;
pub mod config;
pub mod executor;
pub mod flow_storage;
pub mod model;
pub mod tracing_mw;

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

async fn start_server(config: InternalAuthustConfiguration, pool: PgPool) {
    let listener = TcpListener::bind(&config.listen.http);
    let storage = FlowStorage::new(pool.clone());
    let executor = FlowExecutor::new(storage);
    let handlebars = setup_handlebars();
    let app = Route::new()
        .at("/test", poem::get(hello_world))
        .nest("/api/v1", setup_api_v1(&config.secret, pool).await)
        .with(AddData::new(executor))
        .with(AddData::new(handlebars))
        .with(
            SensitiveHeader::new()
                .header(AUTHORIZATION)
                .header(CSRF_HEADER),
        )
        .with(Tracing);
    let server = Server::new(listener).run(app).await.unwrap();
    info!("Listening on {}...", config.listen.http);
    // server.await.expect("Server crashed");
}

#[handler]
async fn hello_world() -> &'static str {
    "Hello, World!"
}
