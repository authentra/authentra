use axum::Router;

pub fn setup_router() -> Router {
    let middlewares = tower::ServiceBuilder::new().layer(crate::telemetry::middleware::new());
    Router::new().layer(middlewares)
}
