use axum::Router;
use tower::ServiceBuilder;
use tracing::instrument;

use crate::AppState;
mod auth;
mod user;

pub fn setup_router() -> Router<AppState> {
    let middlewares = ServiceBuilder::new().layer(crate::telemetry::middleware::new());
    Router::new()
        .nest("/api/auth", auth::router())
        .nest("/api/user", user::router())
        .layer(middlewares)
}
#[instrument]
async fn hello_world() -> &'static str {
    "Hello, World!"
}
