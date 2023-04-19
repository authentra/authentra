use axum::Router;
use tower::ServiceBuilder;

pub fn setup_router() -> Router {
    let middlewares = ServiceBuilder::new().layer(crate::telemetry::middleware::new());
    Router::new().layer(middlewares)
}
