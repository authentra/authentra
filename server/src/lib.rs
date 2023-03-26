pub mod api;
pub mod auth;
pub mod config;
pub mod executor;
pub mod interface;
mod otel_middleware;
pub mod service;
mod state;
use axum::BoxError;
use http::StatusCode;
use once_cell::sync::Lazy;
pub use state::*;
pub mod setup;
pub mod tracing_setup;

#[doc(hidden)]
pub mod test_util;

pub static INTERFACE_BASE_URI: Lazy<&'static str> = Lazy::new(base_uri);

fn base_uri() -> &'static str {
    let env = std::env::var("INTERFACE_BASE_URI").ok();
    match env {
        Some(v) => {
            tracing::info!("Serving web interface from url '{}' ", v);
            Box::leak(v.into_boxed_str())
        }
        None => {
            tracing::info!("Serving web interface from files");
            ""
        }
    }
}
pub async fn hello_world() -> &'static str {
    "Hello, World!"
}
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations/");
}
async fn handle_timeout_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", err),
        )
    }
}
