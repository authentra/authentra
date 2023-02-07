
use axum_extra::extract::{cookie::Cookie, CookieJar};
use http::StatusCode;
use sqlx::{query, Postgres};
use tracing::instrument;

use crate::api::{sql_tx::Tx, v1::auth::Session, v1::auth::SESSION_COOKIE_NAME, ApiError};

#[instrument(skip(tx, session, cookies))]
pub async fn logout(
    session: Option<Session>,
    mut tx: Tx<Postgres>,
    cookies: CookieJar,
) -> Result<(CookieJar, StatusCode), ApiError> {
    let cookies = if let Some(session) = session {
        query!("delete from sessions where uid = $1", session.session_id)
            .execute(&mut tx)
            .await?;
        cookies.remove(Cookie::named(SESSION_COOKIE_NAME))
    } else {
        cookies
    };
    Ok((cookies, StatusCode::OK))
}
