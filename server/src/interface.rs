use axum::{routing::get_service, Router};
use tower_http::services::{ServeDir, ServeFile};

use crate::SharedState;

use self::flow::setup_flow_router;

mod flow;

pub fn setup_interface_router() -> Router<SharedState> {
    let spa = spa_router();
    Router::new()
        .nest("/flow/-", setup_flow_router())
        .merge(spa)
}

fn spa_router() -> Router<SharedState> {
    let asset_service = get_service(ServeDir::new("dist/assets"));
    let favicon_service = get_service(ServeFile::new("dist/favicon.ico"));
    let fallback_service = get_service(ServeFile::new("dist/index.html"));
    Router::new()
        .nest_service("/assets", asset_service)
        .route_service("/favicon.ico", favicon_service)
        .fallback_service(fallback_service)
}
