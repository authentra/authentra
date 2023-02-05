use argon2::{password_hash::SaltString, Params, PasswordHasher};
use http::StatusCode;
use poem::{handler, web::Form, IntoResponse, Response};
use rand::rngs::OsRng;
use serde::Deserialize;
use sqlx::{query, Postgres};

use crate::api::sql_tx::Tx;

#[derive(Deserialize)]
pub struct RegisterForm {
    name: String,
    password: String,
}

#[handler]
pub async fn register(
    mut tx: Tx<Postgres>,
    Form(form): Form<RegisterForm>,
) -> poem::Result<Response> {
    let name = form.name;
    let salt = SaltString::generate(&mut OsRng);
    let password = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::default(),
    )
    .hash_password(form.password.as_bytes(), &salt)
    .map_err(|err| {
        tracing::error!("Failed to hash password {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .to_string();
    let uid = query!(
        "insert into users(name, password) values($1, $2) returning uid",
        name,
        password
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .uid;
    Ok(StatusCode::OK.into_response())
}
