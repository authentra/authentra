use http::StatusCode;
use sqlx::{query, Postgres};
use tower_cookies::{Cookie, Cookies};
use tracing::instrument;

use crate::api::{sql_tx::Tx, v1::auth::Session, v1::auth::SESSION_COOKIE_NAME, ApiError};

//TODO: Optional session resolver to prevent creating new sessions when not needed

#[instrument(skip(tx, session, cookies))]
pub async fn logout(
    session: Option<Session>,
    mut tx: Tx<Postgres>,
    cookies: Cookies,
) -> Result<StatusCode, ApiError> {
    if let Some(session) = session {
        query!("delete from sessions where uid = $1", session.session_id)
            .execute(&mut tx)
            .await?;
        cookies.remove(Cookie::named(SESSION_COOKIE_NAME));
    }
    Ok(StatusCode::OK)
}
