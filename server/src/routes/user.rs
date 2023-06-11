use axum::{extract::State, routing::get, Router};
use serde::Serialize;
use tracing::instrument;

use crate::{
    auth::{ApiAuth, UserRole},
    ApiResponse, AppResult, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/@me", get(me))
}

#[derive(Serialize)]
struct EncodedUser {
    name: String,
    roles: Vec<UserRole>,
}

#[instrument(skip_all, name = "user_me_request_handler")]
async fn me(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
) -> AppResult<ApiResponse<EncodedUser>> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select name,roles from users where uid = $1")
        .await?;
    let row = conn.query_one(&stmt, &[&info.user]).await?;
    Ok(ApiResponse(EncodedUser {
        name: row.get("name"),
        roles: row.get("roles"),
    }))
}
