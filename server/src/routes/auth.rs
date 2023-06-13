use axum::{
    extract::State,
    http::request::Parts,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use deadpool_postgres::{GenericClient, Object};
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use serde::Deserialize;
use tokio_postgres::IsolationLevel;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    auth::{jwt_header, AuthError, AuthustClaims, Claims, CookieAuth, SESSION_COOKIE},
    utils::password::{handle_result, hash_password, verify_password},
    ApiResponse, AppResult, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/browser/refresh", get(refresh))
        .route("/browser/login", post(browser_login))
        .route("/browser/register", post(register))
        .route("/browser/logout", delete(logout))
        .route("/login", post(api_login))
        .route("/registration", get(registration_enabled))
}

#[derive(Deserialize)]
pub struct LoginPayload {
    user: String,
    password: String,
}
#[derive(Deserialize)]
pub struct RegisterPayload {
    user: String,
    password: String,
}

fn failed<T>() -> AppResult<T> {
    Err(AuthError::InvalidCredentials.into())
}

async fn registration_enabled() -> AppResult<ApiResponse<bool>> {
    Ok(ApiResponse(true))
}

#[instrument(skip_all, name = "internal_login_handler")]
async fn handle_login(
    conn: &impl GenericClient,
    payload: LoginPayload,
) -> AppResult<ApiResponse<String>> {
    let stmt = conn
        .prepare_cached("select uid,password from users where name = $1")
        .await?;
    let row = conn.query_opt(&stmt, &[&payload.user]).await?;
    match row {
        Some(row) => {
            let uid: Uuid = row.get("uid");
            let Some(password): Option<String> = row.get("password") else { return failed() };
            let passed = tokio::task::spawn_blocking(move || {
                handle_result(verify_password(
                    password.as_str(),
                    payload.password.as_bytes(),
                ))
            })
            .await??;
            let token = {
                let mut rng = thread_rng();
                Alphanumeric.sample_string(&mut rng, 255)
            };
            let stmt = conn
                .prepare_cached("insert into sessions(user_id,token,address) values($1, $2, null)")
                .await?;
            conn.execute(&stmt, &[&uid, &token]).await?;
            // passed.map_or_else(
            //     || failed(),
            //     |_| Ok((make_cookies(token), ApiResponse(())).into_response()),
            // )
            passed.map_or_else(|| failed(), |_| Ok(ApiResponse(token)))
            // todo!()
        }
        None => failed(),
    }
}
#[instrument(skip_all, name = "api_login_request_handler")]
async fn api_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> AppResult<ApiResponse<String>> {
    let conn = state.conn().await?;
    handle_login(&conn, payload).await
}

#[instrument(skip_all, name = "browser_login_request_handler")]
async fn browser_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> AppResult<Response> {
    let conn = state.conn().await?;
    let v = handle_login(&conn, payload).await?;
    Ok((make_cookies(v.0), ApiResponse(())).into_response())
}

fn make_cookies(token: String) -> CookieJar {
    let jar = CookieJar::new();
    let mut cookie = Cookie::new(SESSION_COOKIE, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_secure(false);
    cookie.set_same_site(SameSite::None);
    jar.add(cookie)
}

#[instrument(skip_all, name = "register_request_handler")]
async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterPayload>,
) -> AppResult<ApiResponse<()>> {
    let hashed =
        tokio::task::spawn_blocking(move || hash_password(payload.password.as_bytes())).await??;
    let mut conn = state.conn().await?;
    let tx = conn
        .build_transaction()
        .isolation_level(IsolationLevel::Serializable)
        .start()
        .await?;
    let stmt = tx
        .prepare_cached("insert into users(name,password) values($1, $2) on conflict do nothing")
        .await?;
    let _modified = tx.execute(&stmt, &[&payload.user, &hashed]).await?;
    tx.commit().await?;
    Ok(ApiResponse(()))
}

async fn logout(State(state): State<AppState>, parts: Parts) -> AppResult<Response> {
    let cookies = CookieJar::from_headers(&parts.headers);
    let Some(session) = cookies.get(SESSION_COOKIE) else { return Ok(().into_response()) };
    let value = session.value();
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("delete from sessions where token = $1")
        .await?;
    conn.execute(&stmt, &[&value]).await?;
    Ok((
        cookies.remove(Cookie::named(SESSION_COOKIE)),
        ApiResponse(()),
    )
        .into_response())
}

#[instrument(skip_all, name = "auth_refresh_handler")]
async fn refresh(
    State(state): State<AppState>,
    CookieAuth(info): CookieAuth,
) -> AppResult<ApiResponse<String>> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select roles from users where uid = $1")
        .await?;
    let row = conn.query_one(&stmt, &[&info.user]).await?;
    let authust = AuthustClaims {
        roles: row.get("roles"),
    };
    let claims = Claims::new(info.user, info.id, authust);
    let token = jsonwebtoken::encode(&jwt_header(), &claims, state.auth().encoding())?;
    Ok(ApiResponse(token))
}
