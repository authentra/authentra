use poem::{get, Route};

use super::{executor::setup_executor_router, ping_handler};

pub fn setup_flow_router() -> Route {
    Route::new()
        .at("/ping", get(ping_handler))
        // .at("/executor", get(ping_handler))
        .at("/executor", setup_executor_router())
}
