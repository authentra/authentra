use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tracing::instrument;

use crate::middleware::auth::AuthState;

pub fn setup_router(auth_state: AuthState) -> Router {
    let middlewares = ServiceBuilder::new()
        .layer(crate::telemetry::middleware::new())
        .layer(CookieManagerLayer::new())
        .layer(crate::middleware::auth::AuthLayer::new(auth_state));
    Router::new()
        .route("/", get(hello_world))
        .layer(middlewares)
}

#[instrument]
async fn hello_world() -> &'static str {
    "Hello, World!"
}
