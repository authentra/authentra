use axum::{extract::State, routing::get, Router};
use serde::Serialize;

use crate::{auth::ApiAuth, ApiResponse, AppResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/@me", get(me))
}

#[derive(Serialize)]
struct EncodedUser {
    name: String,
}

async fn me(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
) -> AppResult<ApiResponse<EncodedUser>> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select name from users where uid = $1")
        .await?;
    let row = conn.query_one(&stmt, &[&info.user]).await?;
    Ok(ApiResponse(EncodedUser {
        name: row.get("name"),
    }))
}
