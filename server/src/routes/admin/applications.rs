use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::MethodRouter,
    Json, Router,
};
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::ApiAuth, error::ErrorKind, ApiResponse, AppResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", MethodRouter::new().get(get).post(create))
        .route("/:id", MethodRouter::new().put(replace).delete(delete))
}
#[derive(Debug, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "application_kind")]
pub enum ApplicationKind {
    #[postgres(name = "web-server")]
    #[serde(rename = "web-server")]
    WebServer,
    #[postgres(name = "spa")]
    #[serde(rename = "spa")]
    SPA,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncodedApplication {
    id: Uuid,
    name: String,
    application_group: String,
    kind: ApplicationKind,
    client_id: String,
    redirect_uri: Vec<String>,
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ApiAuth(auth): ApiAuth,
) -> AppResult<ApiResponse<()>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("delete from applications where id = $1")
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
) -> AppResult<ApiResponse<Vec<EncodedApplication>>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached(
            "select id,name,application_group,kind,client_id,redirect_uri from applications",
        )
        .await?;
    let rows = conn.query(&stmt, &[]).await?;
    Ok(ApiResponse(
        rows.into_iter()
            .map(|row| EncodedApplication {
                id: row.get("id"),
                name: row.get("name"),
                application_group: row.get("application_group"),
                kind: row.get("kind"),
                client_id: row.get("client_id"),
                redirect_uri: row.get("redirect_uri"),
            })
            .collect(),
    ))
}

#[derive(Serialize, Deserialize)]
pub struct ReplacePayload {
    name: String,
    redirect_uri: Vec<String>,
}

async fn replace(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReplacePayload>,
) -> AppResult<ApiResponse<EncodedApplication>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("update applications set name = $2, redirect_uri = $3 where id = $1")
        .await?;
    let row = conn
        .execute(&stmt, &[&id, &payload.name, &payload.redirect_uri])
        .await?;
    if row == 0 {
        return Err(ErrorKind::Status(StatusCode::NOT_FOUND).into());
    } else if row > 1 {
        tracing::error!("Updated more than one row! Payload: {:?}", id);
        return Err(ErrorKind::Status(StatusCode::INTERNAL_SERVER_ERROR).into());
    } else {
        let stmt = conn
            .prepare_cached(
                "select id,name,application_group,kind,client_id,redirect_uri from applications where id = $1",
            )
            .await?;
        let row = conn.query_one(&stmt, &[&id]).await?;
        Ok(ApiResponse(EncodedApplication {
            id: row.get("id"),
            name: row.get("name"),
            application_group: row.get("application_group"),
            kind: row.get("kind"),
            client_id: row.get("client_id"),
            redirect_uri: row.get("redirect_uri"),
        }))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CreatePayload {
    name: String,
    application_group: String,
    kind: ApplicationKind,
    #[serde(default)]
    redirect_uri: Vec<String>,
}

async fn create(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    Json(payload): Json<CreatePayload>,
) -> AppResult<ApiResponse<EncodedApplication>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("insert into applications(name,application_group, kind, redirect_uri,client_secret) values($1,$2,$3,$4,$5) on conflict do nothing returning *")
        .await?;
    let row = conn
        .query_one(
            &stmt,
            &[
                &payload.name,
                &payload.application_group,
                &payload.kind,
                &payload.redirect_uri,
                &None::<String>,
            ],
        )
        .await?;
    Ok(ApiResponse(EncodedApplication {
        id: row.get("id"),
        name: row.get("name"),
        application_group: row.get("application_group"),
        kind: row.get("kind"),
        client_id: row.get("client_id"),
        redirect_uri: row.get("redirect_uri"),
    }))
}
