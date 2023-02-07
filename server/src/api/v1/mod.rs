use axum::{routing::get, Router};

use sqlx::PgPool;
use tower::ServiceBuilder;

use crate::{
    api::{
        csrf::CsrfLayer,
        sql_tx::TxLayer,
        v1::{auth::setup_auth_router, flow::setup_flow_router},
    },
    AppState,
};

use self::auth::AuthLayer;

pub mod application;
pub mod auth;
pub mod executor;
pub mod flow;

pub async fn setup_api_v1(_secret: &str, state: AppState, pool: PgPool) -> Router {
    let service = ServiceBuilder::new()
        .layer(TxLayer::new(pool))
        .layer(CsrfLayer::new(vec!["*".into()]))
        .layer(AuthLayer::new(state.auth_data.clone()));
    let router = Router::new()
        .route("/ping", get(ping_handler))
        .nest("/flow", setup_flow_router())
        .nest("/auth", setup_auth_router())
        .layer(service);
    router
}

async fn ping_handler() -> &'static str {
    "Pong!"
}
