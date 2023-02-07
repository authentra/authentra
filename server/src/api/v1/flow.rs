use axum::{routing::get, Router};

use super::{executor::setup_executor_router, ping_handler};

pub fn setup_flow_router() -> Router {
    Router::new()
        .route("/ping", get(ping_handler))
        // .at("/executor", get(ping_handler))
        .nest("/executor", setup_executor_router())
}
