use argon2::{Argon2, PasswordHash};
use axum::Form;
use http::StatusCode;
use serde::Deserialize;
use sqlx::{query, Postgres};
use tower_cookies::Cookies;
use tracing::instrument;

use crate::api::{sql_tx::Tx, ApiError};

#[derive(Deserialize)]
pub struct LoginForm {
    name: String,
    password: String,
}

#[instrument(skip(tx, form, _cookies))]
pub async fn login(
    mut tx: Tx<Postgres>,
    _cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<StatusCode, ApiError> {
    let rec = query!("select uid,password from users where name = $1", form.name)
        .fetch_optional(&mut tx)
        .await?;
    Ok(match rec {
        Some(rec) => {
            let hash =
                PasswordHash::parse(&rec.password, argon2::password_hash::Encoding::default())?;
            hash.verify_password(&[&Argon2::default()], form.password.as_bytes())?;
            StatusCode::OK
        }
        None => StatusCode::UNAUTHORIZED,
    })
}
