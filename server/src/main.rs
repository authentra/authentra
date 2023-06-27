use std::{net::SocketAddr, ops::DerefMut, process::exit};

use axum::{
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts, Query},
    http::request::Parts,
    response::IntoResponse,
    Json, Server,
};
use deadpool_postgres::{Config, Object, Pool};
use error::Error;
use serde::{de::DeserializeOwned, Serialize};
use tracing::info;

use crate::{auth::AuthState, config::AuthentraConfiguration};

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

pub const PAGE_LIMIT: u16 = 100;

macro_rules! api_extractor {
    ($name:ident, $error:ty, $ty:tt) => {
        pub struct $name<T>(T);

        #[axum::async_trait]
        impl<T, S, B> FromRequest<S, B> for $name<T>
        where
            $ty<T>: FromRequest<S, B, Rejection = $error>,
            S: Send + Sync,
            B: Send + 'static,
        {
            type Rejection = Error;
            async fn from_request(
                req: axum::http::Request<B>,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let extractor = $ty::from_request(req, state).await?;
                Ok(Self(extractor.0))
            }
        }
    };
}

pub struct ApiQuery<T>(T);

#[axum::async_trait]
impl<T: DeserializeOwned, S: Send + Sync> FromRequestParts<S> for ApiQuery<T> {
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let extractor = Query::from_request_parts(parts, state).await?;
        Ok(Self(extractor.0))
    }
}

api_extractor!(ApiJson, JsonRejection, Json);

#[cfg(not(unix))]
async fn shutdown_future() {
    signal::ctrl_c().await.unwrap()
}
#[cfg(unix)]
async fn shutdown_future() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut term = signal(SignalKind::terminate()).unwrap();
    let mut int = signal(SignalKind::interrupt()).unwrap();
    tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
    };
}

async fn main_tokio() {
    let configuration = AuthentraConfiguration::load().unwrap();
    telemetry::setup_tracing();

    let pool = create_database_pool(configuration.postgres.clone());
    {
        let mut conn = pool.get().await.expect("Failed to get database connection");
        run_migrations(&mut conn).await;
    }
    let auth_state = AuthState::new(configuration.secret.as_str());

    let state = AppState::new(pool, auth_state);

    let router = routes::setup_router().with_state(state);
    Server::bind(&configuration.listen.http)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_future())
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
        .await;
    match report {
        Ok(report) => {
            info!("Applied {} migrations", report.applied_migrations().len());
        }
        Err(err) => {
            tracing::error!("Failed to run migrations! {}", err);
            exit(1)
        }
    }
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
