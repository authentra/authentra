use axum::{routing::get, Router};

use tower::ServiceBuilder;

use crate::{
    api::{
        csrf::CsrfLayer,
        v1::{auth::setup_auth_router, flow::setup_flow_router},
    },
    SharedState,
};

use self::{auth::AuthLayer, executor::setup_executor_router, policy::setup_policy_router};

pub mod application;
pub mod auth;
pub mod executor;
pub mod flow;
pub mod policy;

pub async fn setup_api_v1(state: SharedState) -> Router<SharedState> {
    let service = ServiceBuilder::new()
        .layer(CsrfLayer::new(vec!["*".into()]))
        .layer(AuthLayer::new(state.auth_data().clone()));
    
    Router::new()
        .route("/ping", get(ping_handler))
        .nest("/flows", setup_flow_router())
        .nest("/flow/executor", setup_executor_router())
        .nest("/auth", setup_auth_router())
        .nest("/policies", setup_policy_router())
        .layer(service)
}

async fn ping_handler() -> &'static str {
    "Pong!"
}
