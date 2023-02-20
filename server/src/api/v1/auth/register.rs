use argon2::{password_hash::SaltString, Params, PasswordHasher};
use axum::{
    response::{IntoResponse, Response},
    Form,
};
use http::StatusCode;
use rand::rngs::OsRng;
use serde::Deserialize;
use sqlx::{query, Postgres};
use tracing::instrument;

use crate::api::{sql_tx::Tx, ApiError};

#[derive(Deserialize)]
pub struct RegisterForm {
    name: String,
    password: String,
}

#[instrument(skip(tx, form))]
pub async fn register(
    mut tx: Tx<Postgres>,
    Form(form): Form<RegisterForm>,
) -> Result<Response, ApiError> {
    let name = form.name;
    let salt = SaltString::generate(&mut OsRng);
    let password = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::default(),
    )
    .hash_password(form.password.as_bytes(), &salt)?
    .to_string();
    tracing::trace!("Executing query");
    let _uid = query!(
        "insert into users(name, password) values($1, $2) returning uid",
        name,
        password
    )
    .fetch_one(&mut tx)
    .await?
    .uid;
    tracing::trace!("Executed query");
    Ok(StatusCode::OK.into_response())
}
