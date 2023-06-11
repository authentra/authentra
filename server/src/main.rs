use std::{net::SocketAddr, ops::DerefMut};

use axum::{response::IntoResponse, Json, Server};
use deadpool_postgres::{Config, Object, Pool};
use serde::Serialize;
use tokio::signal;
use tracing::info;

use crate::{auth::AuthState, config::AuthustConfiguration};

pub mod auth;
mod config;
pub mod routes;
mod state;
pub use state::AppState;
pub mod error;
mod telemetry;
pub mod utils;

#[tokio::main]
async fn main() {
    main_tokio().await;
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/");
}

pub type AppResult<T, E = error::Error> = Result<T, E>;

async fn main_tokio() {
    let configuration = AuthustConfiguration::load().unwrap();
    telemetry::setup_tracing();

    let pool = create_database_pool(configuration.postgres.clone());
    {
        let mut conn = pool.get().await.expect("Failed to get database connection");
        run_migrations(&mut conn).await;
    }
    let auth_state = AuthState::new(configuration.secret.as_str());

    let state = AppState::new(pool, auth_state);

    let router = routes::setup_router().with_state(state);
    let shutdown_future = async { signal::ctrl_c().await.unwrap() };
    Server::bind(&configuration.listen.http)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_future)
        .await
        .expect("Server crashed");
    info!("Server shutdown");
    opentelemetry::global::shutdown_tracer_provider();
}

pub fn create_database_pool(configuration: Config) -> Pool {
    configuration
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("Failed to create pool")
}

pub async fn run_migrations(client: &mut Object) {
    info!("Running migrations on database...");
    let report = embedded::migrations::runner()
        .run_async(client.as_mut().deref_mut())
        .await
        .expect("Failed to run migrations");
    info!("Applied {} migrations", report.applied_migrations().len());
}

pub struct ApiResponse<T>(T);

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(InternalApiResponse {
            success: true,
            response: self.0,
        })
        .into_response()
    }
}

#[derive(Serialize)]
struct InternalApiResponse<T> {
    success: bool,
    response: T,
}
