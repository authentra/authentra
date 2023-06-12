use axum::Router;

use crate::AppState;

mod application_groups;
mod applications;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/applications", applications::router())
        .nest("/application-groups", application_groups::router())
}
