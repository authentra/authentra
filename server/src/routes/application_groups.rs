use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::MethodRouter,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{ApiAuth, UserRole},
    error::ErrorKind,
    routes::InternalScope,
    ApiResponse, AppResult, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", MethodRouter::new().get(get).post(create))
        .route("/:id", MethodRouter::new().put(replace).delete(delete))
        .route("/:id/usages", MethodRouter::new().get(usages))
}

#[derive(Debug, Serialize, Deserialize)]
struct EncodedApplicationGroup {
    id: String,
    scopes: Vec<InternalScope>,
}

async fn usages(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    ApiAuth(auth): ApiAuth,
) -> AppResult<ApiResponse<Vec<String>>> {
    auth.check_admin()?;
    //TODO: implement usages
    Ok(ApiResponse(vec![]))
}
async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiAuth(auth): ApiAuth,
) -> AppResult<ApiResponse<()>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("delete from application_groups where id = $1")
        .await?;
    let row = conn.execute(&stmt, &[&id]).await?;
    if row == 0 {
        return Err(ErrorKind::Status(StatusCode::NOT_FOUND).into());
    } else if row > 1 {
        tracing::error!("Updated more than one row when deleting! Id: {:?}", id);
        return Err(ErrorKind::Status(StatusCode::INTERNAL_SERVER_ERROR).into());
    } else {
        Ok(ApiResponse(()))
    }
}

async fn get(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
) -> AppResult<ApiResponse<Vec<EncodedApplicationGroup>>> {
    auth.check_developer()?;
    let conn = state.conn().await?;
    let stmt = if auth.has_role(UserRole::Admin) {
        conn.prepare_cached("select id,scopes from application_groups")
            .await?
    } else {
        conn.prepare_cached("select id,scopes from application_groups where id in (select id from developer_allowed_groups)")
            .await?
    };
    let rows = conn.query(&stmt, &[]).await?;
    Ok(ApiResponse(
        rows.into_iter()
            .map(|row| EncodedApplicationGroup {
                id: row.get("id"),
                scopes: row.get("scopes"),
            })
            .collect(),
    ))
}

#[derive(Serialize, Deserialize)]
pub struct ReplacePayload {
    scopes: Vec<InternalScope>,
}

async fn replace(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    Path(id): Path<String>,
    Json(payload): Json<ReplacePayload>,
) -> AppResult<ApiResponse<EncodedApplicationGroup>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let payload = EncodedApplicationGroup {
        id,
        scopes: payload.scopes,
    };
    let stmt = conn
        .prepare_cached("update application_groups set scopes = $2 where id = $1")
        .await?;
    let row = conn.execute(&stmt, &[&payload.id, &payload.scopes]).await?;
    if row == 0 {
        return Err(ErrorKind::Status(StatusCode::NOT_FOUND).into());
    } else if row > 1 {
        tracing::error!("Updated more than one row! Payload: {:?}", payload);
        return Err(ErrorKind::Status(StatusCode::INTERNAL_SERVER_ERROR).into());
    } else {
        Ok(ApiResponse(payload))
    }
}
async fn create(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    Json(payload): Json<EncodedApplicationGroup>,
) -> AppResult<ApiResponse<EncodedApplicationGroup>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached(
            "insert into application_groups(id, scopes) values($1, $2) on conflict do nothing",
        )
        .await?;
    let row = conn.execute(&stmt, &[&payload.id, &payload.scopes]).await?;
    if row == 0 {
        return Err(ErrorKind::Status(StatusCode::CONFLICT).into());
    } else if row > 1 {
        tracing::error!("Updated more than one row! Payload: {:?}", payload);
        return Err(ErrorKind::Status(StatusCode::INTERNAL_SERVER_ERROR).into());
    } else {
        Ok(ApiResponse(payload))
    }
}
