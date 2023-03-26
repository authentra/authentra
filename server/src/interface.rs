use axum::Router;

use crate::SharedState;

use self::flow::setup_flow_router;

mod flow;

pub fn setup_interface_router() -> Router<SharedState> {
    let router = Router::new().nest("/flow/-", setup_flow_router());
    #[cfg(feature = "dev-mode")]
    {
        router
    }
    #[cfg(not(feature = "dev-mode"))]
    {
        let spa = spa_router();
        router.merge(spa)
    }
}

#[cfg(not(feature = "dev-mode"))]
fn spa_router() -> Router<SharedState> {
    use axum::routing::get_service;
    use tower_http::services::{ServeDir, ServeFile};

    let asset_service = get_service(ServeDir::new("dist/assets"));
    let favicon_service = get_service(ServeFile::new("dist/favicon.ico"));
    let fallback_service = get_service(ServeFile::new("dist/index.html"));
    Router::new()
        .nest_service("/assets", asset_service)
        .route_service("/favicon.ico", favicon_service)
        .fallback_service(fallback_service)
}
