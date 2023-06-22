//TODO: Disable CORS for oauth2

use std::{collections::HashMap, str::FromStr};

use axum::{
    extract::State,
    http::{Method, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use derive_more::{Display, From};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{formats::SpaceSeparator, serde_as, StringWithSeparator};
use url::Url;
use uuid::Uuid;

use crate::{
    auth::ApiAuth,
    error::{ErrorKind, IntoError},
    ApiQuery, ApiResponse, AppResult, AppState,
};

use super::InternalScope;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseType {
    Code,
    Token,
    Unknown(String),
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    #[default]
    Query,
    FormPost,
    Fragment,
    Unknown(String),
}

#[serde_as]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct OAuthParameters {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub response_type: ResponseType,
    #[serde(default)]
    pub response_mode: ResponseMode,
    pub redirect_uri: String,
    #[serde(rename = "scope")]
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scopes: Vec<String>,
    pub state: Option<String>,
    #[serde(flatten)]
    pub other: HashMap<String, String>,
}

#[serde_as]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ConsentInfo {
    pub given: bool,
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scopes: Vec<String>,
}

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/authorize", get(authorize_request).post(authorize_request))
}

#[derive(Debug, Display, From)]
pub enum OAuthError {
    #[display("Unknown client id")]
    UnknownApplication,
    #[display("Unknown redirect uri")]
    UnknownRedirectUri,
    #[display("Invalid client_secret")]
    InvalidClientSecret,
    #[display("Scope: {}", _0)]
    InvalidScope(ScopeFromStrError),
}

#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    #[display("{}", _0)]
    Internal(InternalScope),
    #[display("{}", _0)]
    Custom(String),
}

#[derive(Debug, Display, Serialize)]
pub enum ScopeFromStrError {
    #[display("Empty")]
    Empty,
    #[display("Invalid")]
    Invalid,
}

pub static SCOPE_VALIDATION: Lazy<Regex> = Lazy::new(|| Regex::new("^[a-z\\-.:]$").unwrap());

impl FromStr for Scope {
    type Err = ScopeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ScopeFromStrError::Empty);
        }
        if s.contains(" ") {
            return Err(ScopeFromStrError::Invalid);
        }
        if let Some(internal) = InternalScope::from_str(s) {
            return Ok(Self::Internal(internal));
        }
        if let Some(m) = SCOPE_VALIDATION.find(s) {
            return Ok(Self::Custom(m.as_str().into()));
        } else {
            return Err(ScopeFromStrError::Invalid);
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorKind {
    InvalidRequest,
    UnauthorizedClient,
    AccessDenied,
    UnsupportedResponseType,
    InvalidScope,
    ServerError,
    TemporarilyUnavailable,
}

#[derive(Serialize)]
pub struct OAuthErrorData {
    #[serde(rename = "error")]
    kind: OAuthErrorKind,
    description: Option<String>,
    state: Option<String>,
}

pub async fn authorize_request(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    method: Method,
    ApiQuery(parameters): ApiQuery<OAuthParameters>,
) -> AppResult<Response> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select * from applications where client_id = $1")
        .await?;
    let application = conn
        .query_opt(&stmt, &[&parameters.client_id])
        .await?
        .ok_or_else(|| OAuthError::UnknownApplication.into_error())?;
    let uri = Url::from_str(&parameters.redirect_uri)
        .map_err(|_| OAuthError::UnknownRedirectUri.into_error())?;
    {
        let uris: Vec<String> = application.get("redirect_uri");
        if !uris.contains(&parameters.redirect_uri) {
            return Err(OAuthError::UnknownRedirectUri.into());
        }
    }
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
    println!("response_type: {:?}", parameters.response_type);
    match parameters.response_type {
        ResponseType::Code => {}
        _ => {
            return Ok(make_oauth_error(
                OAuthErrorKind::UnsupportedResponseType,
                None,
                parameters.state,
                uri,
            ))
        }
    }
    match parameters.response_mode {
        ResponseMode::Query => {}
        // ResponseMode::Fragment => {}
        _ => {
            return Ok(make_oauth_error(
                OAuthErrorKind::InvalidRequest,
                None,
                parameters.state,
                uri,
            ))
        }
    }
    let stmt = conn
        .prepare_cached("select * from application_groups where id = $1")
        .await?;
    let application_group = conn
        .query_one(&stmt, &[&application.get::<_, String>("application_group")])
        .await?;
    let application_internal_scopes: Vec<InternalScope> = application_group.get("scopes");
    let (scopes, scope_errors) = find_scopes(
        parameters.scopes.clone().into_iter(),
        &application_internal_scopes,
    );
    match method {
        Method::GET => {
            return Ok(ApiResponse(OAuthResponse::Get {
                app_name: application.get("name"),
                invalid_scopes: scope_errors,
                scopes: scopes.iter().map(ToString::to_string).collect(),
            })
            .into_response());
        }
        Method::POST => {
            let (selected_scopes, _) = find_scopes(
                parameters.other.keys().cloned().filter_map(|s| {
                    if s.starts_with("enable-scope:") {
                        let res = s.replacen("enable-scope:", "", 1);
                        if res.is_empty() {
                            None
                        } else {
                            Some(res)
                        }
                    } else {
                        None
                    }
                }),
                &application_internal_scopes,
            );
            let denied = parameters.other.contains_key("denied");
            if denied {
                return Ok(make_oauth_error(
                    OAuthErrorKind::AccessDenied,
                    None,
                    parameters.state,
                    uri,
                ));
            }
            let selected_scopes: Vec<String> =
                selected_scopes.into_iter().map(|s| s.to_string()).collect();
            let stmt = conn
                .prepare_cached(
                    "insert into authorization_codes(user_id,application,redirect_uri,scope) values($1,$2,$3,$4) returning code",
                )
                .await?;
            let code: String = conn
                .query_one(
                    &stmt,
                    &[
                        &auth.user,
                        &application.get::<_, Uuid>("id"),
                        &parameters.redirect_uri,
                        &selected_scopes.join(" "),
                    ],
                )
                .await?
                .get(0);
            match parameters.response_mode {
                ResponseMode::Query => {
                    let query = encode_query(CodeRedirect {
                        code: Some(code),
                        state: parameters.state,
                    })?;
                    let mut uri = uri;
                    uri.set_query(Some(query.as_str()));
                    return Ok(Redirect::temporary(uri.as_str()).into_response());
                }
                ResponseMode::Fragment => todo!(),
                _ => {
                    return Err(ErrorKind::internal().into());
                }
            }
        }
        _ => return Err(ErrorKind::Status(StatusCode::METHOD_NOT_ALLOWED).into()),
    }
}

#[derive(Serialize)]
struct CodeRedirect {
    code: Option<String>,
    state: Option<String>,
}

fn encode_query<T: Serialize>(t: T) -> AppResult<String> {
    let query = serde_urlencoded::to_string(t);
    match query {
        Ok(v) => Ok(v),
        Err(_) => Err(ErrorKind::internal().into()),
    }
}

fn make_oauth_error(
    kind: OAuthErrorKind,
    description: Option<String>,
    state: Option<String>,
    mut redirect_uri: Url,
) -> Response {
    let query = serde_urlencoded::to_string(OAuthErrorData {
        kind,
        description,
        state,
    });
    let query = match query {
        Ok(v) => v,
        Err(_) => return ErrorKind::internal().into_error().into_response(),
    };
    redirect_uri.set_query(Some(&query));
    Redirect::temporary(redirect_uri.as_str()).into_response()
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum OAuthResponse {
    Get {
        app_name: String,
        invalid_scopes: Vec<String>,
        scopes: Vec<String>,
    },
}

pub fn find_scopes(
    scopes: impl Iterator<Item = String>,
    allowed_scopes: &Vec<InternalScope>,
) -> (Vec<Scope>, Vec<String>) {
    let mut scope_errors = Vec::new();
    // let scopes: Vec<String> = scopes.collect();
    let mut scopes: Vec<Scope> = scopes
        .map(|s| (s.clone(), Scope::from_str(s.as_str())))
        .filter_map(|v| match v.1 {
            Ok(v) => Some(v),
            Err(_err) => {
                scope_errors.push(v.0.clone());
                None
            }
        })
        .collect();
    let mut forbidden_scopes: Vec<Scope> = Vec::new();
    for scope in &scopes {
        if let Scope::Internal(internal) = scope {
            if !allowed_scopes.contains(internal) {
                forbidden_scopes.push(scope.clone());
            }
        }
    }
    scopes.retain(|s| !forbidden_scopes.contains(s));
    (scopes, scope_errors)
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "grant_type", rename_all = "snake_case")]
pub enum TokenEndpoint {
    AuthorizationCode(TokenAuthorizationCode),
    ClientCredentials(TokenClientCredentials),
    RefreshToken(TokenRefreshToken),
}
#[derive(Serialize, Deserialize)]
pub struct TokenAuthorizationCode {
    code: String,
    redirect_uri: String,
    client_id: String,
}
#[derive(Serialize, Deserialize)]
pub struct TokenClientCredentials {}
#[derive(Serialize, Deserialize)]
pub struct TokenRefreshToken {
    pub refresh_token: String,
    pub scope: Option<String>,
}
