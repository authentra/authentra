use axum::{
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, StatusCode},
    routing::get,
    Router,
};
use deadpool_postgres::GenericClient;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    auth::{ApiAuth, UserRole},
    error::{Error, ErrorKind},
    utils::password::hash_password,
    ApiJson, ApiResponse, AppResult, AppState, PAGE_LIMIT,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/@me", get(me))
        .route("/", get(list).post(create))
        .route("/:id", get(user).delete(delete).put(replace))
}

#[derive(Serialize)]
struct EncodedUser {
    name: String,
    roles: Vec<UserRole>,
    require_password_reset: bool,
}

fn per_page_default() -> u16 {
    25
}

fn page_default() -> u8 {
    1
}

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "page_default")]
    pub page: u8,
    #[serde(default = "per_page_default")]
    pub per_page: u16,
}

impl Pagination {
    pub fn limit(&self, max: u16) -> i64 {
        self.per_page.min(max) as i64
    }

    pub fn offset(&self, max: u16) -> i64 {
        (self.limit(max) as i64).saturating_mul((self.page.saturating_sub(1)) as i64)
    }
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Pagination {
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let query: Pagination = Query::from_request_parts(parts, state).await?.0;
        Ok(query)
    }
}

#[derive(Serialize)]
pub struct AdminUser {
    id: Uuid,
    name: String,
    email: Option<String>,
    active: bool,
    roles: Vec<UserRole>,
    customer: bool,
    require_password_reset: bool,
}

fn admin_from_row(row: Row) -> AdminUser {
    AdminUser {
        id: row.get("id"),
        email: row.get("email"),
        name: row.get("name"),
        active: row.get("active"),
        roles: row.get("roles"),
        customer: row.get("customer"),
        require_password_reset: row.get("require_password_reset"),
    }
}

#[instrument(skip_all, name = "user_me_request_handler")]
async fn me(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
) -> AppResult<ApiResponse<EncodedUser>> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select name,roles,require_password_reset from users where id = $1")
        .await?;
    let row = conn.query_one(&stmt, &[&info.user]).await?;
    Ok(ApiResponse(EncodedUser {
        name: row.get("name"),
        roles: row.get("roles"),
        require_password_reset: row.get("require_password_reset"),
    }))
}

#[derive(Deserialize)]
struct CreatePayload {
    name: String,
    password: String,
    roles: Vec<UserRole>,
}

async fn create(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
    ApiJson(payload): ApiJson<CreatePayload>,
) -> AppResult<ApiResponse<()>> {
    info.check_admin()?;
    let conn = state.conn().await?;
    let hashed =
        tokio::task::spawn_blocking(move || hash_password(payload.password.as_bytes())).await??;
    let stmt = conn
        .prepare_cached("insert into users(name,password,require_password_reset,roles,customer) values($1,$2,true,$3,false) on conflict do nothing").await?;
    let rows = conn
        .execute(&stmt, &[&payload.name, &hashed, &payload.roles])
        .await?;
    match rows {
        1 => Ok(ApiResponse(())),
        0 => Err(ErrorKind::Status(StatusCode::CONFLICT).into()),
        i => {
            tracing::error!("Modified rows is not 1 or 0. Modified {i} rows!");
            return Err(ErrorKind::internal().into());
        }
    }
}

async fn user(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
    Path(id): Path<Uuid>,
) -> AppResult<ApiResponse<AdminUser>> {
    info.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select * from users where id = $1")
        .await?;
    let row = conn.query_opt(&stmt, &[&id]).await?;
    match row {
        Some(row) => Ok(ApiResponse(admin_from_row(row))),
        None => Err(ErrorKind::not_found().into()),
    }
}

#[instrument(skip_all name = "is_last_admin")]
async fn is_last_admin(client: &impl GenericClient, id: &Uuid) -> AppResult<bool> {
    let stmt = client
        .prepare_cached(
            "select exists (select id from users where id != $1 and 'admin' = any (roles) and active)",
        )
        .await?;
    let row = client.query_one(&stmt, &[id]).await?;
    let is_last_admin: bool = !row.get::<_, bool>(0);
    Ok(is_last_admin)
}

async fn delete(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
    Path(id): Path<Uuid>,
) -> AppResult<ApiResponse<()>> {
    info.check_admin()?;
    let conn = state.conn().await?;
    if is_last_admin(&conn, &id).await? {
        return Err(ErrorKind::forbidden().into());
    }
    let stmt = conn
        .prepare_cached("delete from users where id = $1")
        .await?;
    let rows = conn.execute(&stmt, &[&id]).await?;
    match rows {
        1 => Ok(ApiResponse(())),
        0 => Err(ErrorKind::Status(StatusCode::CONFLICT).into()),
        i => {
            tracing::error!("Modified rows is not 1 or 0. Modified {i} rows!");
            return Err(ErrorKind::internal().into());
        }
    }
}

#[instrument(skip_all name = "user_list")]
async fn list(
    State(state): State<AppState>,
    ApiAuth(info): ApiAuth,
    pagination: Pagination,
) -> AppResult<ApiResponse<Vec<AdminUser>>> {
    info.check_admin()?;
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select * from users limit $1 offset $2")
        .await?;
    let rows = conn
        .query(
            &stmt,
            &[
                &pagination.limit(PAGE_LIMIT),
                &pagination.offset(PAGE_LIMIT),
            ],
        )
        .await?;
    Ok(ApiResponse(rows.into_iter().map(admin_from_row).collect()))
}

#[derive(Debug, Deserialize)]
struct ReplacePayload {
    name: String,
    email: Option<String>,
    active: bool,
    roles: Vec<UserRole>,
    customer: bool,
    require_password_reset: bool,
}

#[instrument(skip_all name = "edit_user")]
async fn replace(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    Path(id): Path<Uuid>,
    ApiJson(payload): ApiJson<ReplacePayload>,
) -> AppResult<ApiResponse<()>> {
    auth.check_admin()?;
    let conn = state.conn().await?;
    if (!payload.active || !payload.roles.contains(&UserRole::Admin))
        && is_last_admin(&conn, &id).await?
    {
        return Err(ErrorKind::forbidden().into());
    }
    let stmt = conn.prepare_cached("update users set name = $2, email = $3, active = $4, roles = $5, customer = $6, require_password_reset = $7 where id = $1").await?;
    let rows = conn
        .execute(
            &stmt,
            &[
                &id,
                &payload.name,
                &payload.email,
                &payload.active,
                &payload.roles,
                &payload.customer,
                &payload.require_password_reset,
            ],
        )
        .await?;
    match rows {
        1 => Ok(ApiResponse(())),
        0 => Err(ErrorKind::Status(StatusCode::CONFLICT).into()),
        i => {
            tracing::error!("Modified rows is not 1 or 0. Modified {i} rows!");
            return Err(ErrorKind::internal().into());
        }
    }
}
