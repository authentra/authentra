use http::StatusCode;
use poem::{handler, web::cookie::CookieJar, IntoResponse, Response};
use sqlx::{query, Postgres};
use tracing::instrument;

use crate::api::{
    sql_tx::Tx,
    v1::auth::Session,
    v1::{auth::SESSION_COOKIE_NAME, ApiError},
};

#[handler]
#[instrument(skip(tx, session, cookies))]
pub async fn logout(
    session: Option<Session>,
    mut tx: Tx<Postgres>,
    cookies: &CookieJar,
) -> Result<Response, ApiError> {
    if let Some(session) = session {
        query!("delete from sessions where uid = $1", session.session_id)
            .execute(&mut tx)
            .await?;
        cookies.remove(SESSION_COOKIE_NAME);
    }
    Ok(StatusCode::OK.into_response())
}
