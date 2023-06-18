//TODO: Disable CORS for oauth2

use axum::{extract::State, http::Method, routing::get, Router};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_with::{formats::SpaceSeparator, serde_as, StringWithSeparator};

use crate::{auth::ApiAuth, error::IntoError, ApiQuery, AppResult, AppState};

use super::InternalScope;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseType {
    Code,
    Token,
}

#[serde_as]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct OAuthParameters {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub response_type: ResponseType,
    pub redirect_uri: String,
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scopes: Vec<String>,
}

#[serde_as]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ConsentInfo {
    pub given: bool,
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scopes: Vec<String>,
}

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/authorize/check", get(authorize_check))
}

#[derive(Debug, Display)]
pub enum OAuthError {
    #[display("Unknown client id")]
    UnknownApplication,
    #[display("Invalid client_secret")]
    InvalidClientSecret,
}

#[axum::debug_handler]
async fn authorize_check(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    method: Method,
    ApiQuery(parameters): ApiQuery<OAuthParameters>,
) -> AppResult<()> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select * from applications where client_id = $1")
        .await?;
    let application = conn
        .query_opt(&stmt, &[&parameters.client_id])
        .await?
        .ok_or_else(|| OAuthError::UnknownApplication.into_error())?;
    let client_secret: Option<String> = application.get("client_secret");
    if let Some(client_secret) = client_secret {
        if let Some(secret) = parameters.client_secret {
            if secret != client_secret {
                return Err(OAuthError::InvalidClientSecret.into());
            }
        } else {
            return Err(OAuthError::InvalidClientSecret.into());
        }
    }

    let stmt = conn
        .prepare_cached("select * from applications where client_id = $1")
        .await?;
    let application_group = conn
        .query_one(&stmt, &[&application.get::<_, String>("application_group")])
        .await?;
    let application_internal_scopes: Vec<InternalScope> = application_group.get("scopes");
    todo!()
}
