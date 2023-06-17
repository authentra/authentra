use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::MethodRouter,
    Json, Router,
};
use deadpool_postgres::GenericClient;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::{
    auth::{ApiAuth, SessionInfo, UserRole},
    error::{ApiError, Error, ErrorKind},
    routes::ApplicationKind,
    ApiResponse, AppResult, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", MethodRouter::new().get(get).post(create))
        .route("/:id", MethodRouter::new().put(replace).delete(delete))
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

pub struct AppInfo {
    pub owner: Uuid,
    pub system: bool,
}

impl AppInfo {
    pub async fn check_by_id(
        client: &impl GenericClient,
        session: &SessionInfo,
        id: &Uuid,
    ) -> AppResult<()> {
        let stmt = client
            .prepare_cached("select owner,system_application from applications where id = $1")
            .await?;
        let app_info: AppInfo = client.query_opt(&stmt, &[id]).await?.try_into()?;
        app_info.check(session)?;
        Ok(())
    }

    pub fn check(&self, session: &SessionInfo) -> AppResult<()> {
        if session.user == self.owner {
            return Ok(());
        } else if self.system && session.has_role(UserRole::Admin) {
            return Ok(());
        }
        Err(ErrorKind::forbidden().into())
    }

    pub fn from_row(row: &Row) -> Self {
        Self {
            owner: row.get("owner"),
            system: row.get("system_application"),
        }
    }
}

impl TryFrom<Option<Row>> for AppInfo {
    type Error = Error;

    fn try_from(value: Option<Row>) -> Result<Self, Self::Error> {
        match value {
            Some(row) => Ok(Self::from_row(&row)),
            None => Err(ErrorKind::not_found().into()),
        }
    }
}

async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ApiAuth(auth): ApiAuth,
) -> AppResult<ApiResponse<()>> {
    auth.check_developer()?;
    let conn = state.conn().await?;
    AppInfo::check_by_id(&conn, &auth, &id).await?;
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
    auth.check_developer()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached(
            "select id,name,application_group,kind,client_id,redirect_uri,owner,system_application from applications where owner = $1 or (system_application and $2)",
        )
        .await?;
    let rows = conn
        .query(&stmt, &[&auth.user, &auth.has_role(UserRole::Admin)])
        .await?;
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
    AppInfo::check_by_id(&conn, &auth, &id).await?;
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
    #[serde(default)]
    system_application: bool,
}

async fn create(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    Json(payload): Json<CreatePayload>,
) -> AppResult<ApiResponse<EncodedApplication>> {
    if payload.system_application {
        auth.check_admin()?;
    } else {
        auth.check_developer()?;
    }
    let conn = state.conn().await?;
    let stmt = if auth.has_role(UserRole::Admin) {
        conn.prepare_cached("select id from application_groups where id = $1")
            .await?
    } else {
        conn.prepare_cached("select id from developer_allowed_groups where id = $1")
            .await?
    };
    let row = conn.query_opt(&stmt, &[&payload.application_group]).await?;
    if row.is_none() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("Unknown application_group '{}'", payload.application_group),
        )
        .into());
    }
    let stmt = conn
        .prepare_cached("insert into applications(name,application_group, owner, kind, redirect_uri,client_secret,consent_mode,system_application) values($1,$2,$3,$4,$5,$6, 'explicit', $7) on conflict do nothing returning *")
        .await?;
    let row = conn
        .query_one(
            &stmt,
            &[
                &payload.name,
                &payload.application_group,
                &auth.user,
                &payload.kind,
                &payload.redirect_uri,
                &None::<String>,
                &payload.system_application,
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
