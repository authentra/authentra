use axum::Router;

use crate::SharedState;

use self::flow::setup_flow_router;

mod flow;

pub fn setup_interface_router() -> Router<SharedState> {
    Router::new().nest("/flow/-", setup_flow_router())
}
