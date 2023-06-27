use std::{collections::HashMap, str::FromStr};

use axum::{
    extract::{FromRequestParts, Query, State},
    http::{request::Parts, Method, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::{formats::SpaceSeparator, serde_as, StringWithSeparator};
use url::Url;
use uuid::Uuid;

use crate::{
    auth::ApiAuth,
    error::{Error, ErrorKind, IntoError},
    ApiResponse, AppResult, AppState,
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
pub struct OAuthAuthorizeParameters {
    pub client_id: String,
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

#[derive(Debug, Display, Clone, Serialize)]
#[display("{self:?}")]
pub struct NewError {
    #[serde(skip)]
    pub state: Option<String>,
    #[serde(rename = "error_description")]
    pub description: Option<String>,
    #[serde(rename = "error")]
    pub kind: OAuthErrorKind,
    #[serde(rename = "error_uri")]
    pub error_uri: Option<String>,
    #[serde(skip)]
    pub redirect_uri: Option<Url>,
}

#[derive(Serialize)]
pub struct NewErrorQueryData {
    #[serde(rename = "error")]
    kind: OAuthErrorKind,
    #[serde(rename = "error_description")]
    description: Option<String>,
    state: Option<String>,
}

impl IntoResponse for NewError {
    fn into_response(self) -> Response {
        if let Some(mut redirect_uri) = self.redirect_uri {
            let query = serde_urlencoded::to_string(NewErrorQueryData {
                kind: self.kind.clone(),
                description: self.description,
                state: self.state,
            });
            let query = match query {
                Ok(v) => v,
                Err(_) => return ErrorKind::internal().into_error().into_response(),
            };
            redirect_uri.set_query(Some(&query));

            Redirect::temporary(redirect_uri.as_str()).into_response()
        } else {
            let status = self.kind.status();
            (status, Json(self)).into_response()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum OAuthErrorKind {
    Common(OAuthErrorCommonKind),
    Authorize(OAuthErrorAuthorizeKind),
    Token(OAuthErrorTokenKind),
    NotSpec(NewErrorNotSpec),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NewErrorNotSpec {
    InvalidClient,
    InvalidRedirectUri,
}

impl OAuthErrorKind {
    pub fn status(&self) -> StatusCode {
        match self {
            OAuthErrorKind::Common(kind) => match kind {
                OAuthErrorCommonKind::InvalidRequest => StatusCode::BAD_REQUEST,
                OAuthErrorCommonKind::UnauthorizedClient => StatusCode::UNAUTHORIZED,
                OAuthErrorCommonKind::InvalidScope => StatusCode::BAD_REQUEST,
            },
            OAuthErrorKind::Authorize(kind) => match kind {
                OAuthErrorAuthorizeKind::AccessDenied => StatusCode::FORBIDDEN,
                OAuthErrorAuthorizeKind::UnsupportedResponseType => StatusCode::BAD_REQUEST,
                OAuthErrorAuthorizeKind::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
                OAuthErrorAuthorizeKind::TemporarilyUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            },
            OAuthErrorKind::Token(kind) => match kind {
                OAuthErrorTokenKind::InvalidClient => StatusCode::UNAUTHORIZED,
                OAuthErrorTokenKind::InvalidGrant => StatusCode::UNAUTHORIZED,
                OAuthErrorTokenKind::UnsupportedGrantType => StatusCode::UNAUTHORIZED,
            },
            OAuthErrorKind::NotSpec(kind) => match kind {
                NewErrorNotSpec::InvalidClient => StatusCode::UNAUTHORIZED,
                NewErrorNotSpec::InvalidRedirectUri => StatusCode::BAD_REQUEST,
            },
        }
    }
}

impl NewError {
    pub fn new(
        kind: OAuthErrorKind,
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self {
            state,
            description,
            kind,
            error_uri,
            redirect_uri,
        }
    }
    pub fn invalid_client(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::NotSpec(NewErrorNotSpec::InvalidClient),
            description,
            state,
            error_uri,
            None,
        )
    }
    pub fn invalid_redirect_uri(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::NotSpec(NewErrorNotSpec::InvalidRedirectUri),
            description,
            state,
            error_uri,
            None,
        )
    }
    pub fn invalid_request(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Common(OAuthErrorCommonKind::InvalidRequest),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn unauthorized_client(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Common(OAuthErrorCommonKind::UnauthorizedClient),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn invalid_scope(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Common(OAuthErrorCommonKind::InvalidScope),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn authorize_access_denied(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Authorize(OAuthErrorAuthorizeKind::AccessDenied),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn authorize_unsupported_response_type(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Authorize(OAuthErrorAuthorizeKind::UnsupportedResponseType),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn token_invalid_client(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Token(OAuthErrorTokenKind::InvalidClient),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn token_invalid_grant(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Token(OAuthErrorTokenKind::InvalidGrant),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
    pub fn token_unsupported_grant_type(
        description: Option<String>,
        state: Option<String>,
        error_uri: Option<String>,
        redirect_uri: Option<Url>,
    ) -> Self {
        Self::new(
            OAuthErrorKind::Token(OAuthErrorTokenKind::UnsupportedGrantType),
            description,
            state,
            error_uri,
            redirect_uri,
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorCommonKind {
    InvalidRequest,
    UnauthorizedClient,
    InvalidScope,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorAuthorizeKind {
    AccessDenied,
    UnsupportedResponseType,
    ServerError,
    TemporarilyUnavailable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorTokenKind {
    InvalidClient,
    InvalidGrant,
    UnsupportedGrantType,
}

#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    #[display("{}", _0)]
    Internal(InternalScope),
    #[display("{}", _0)]
    Custom(String),
}

#[derive(Debug, Display, Serialize)]
pub struct InvalidScopeFromStrError;
pub static SCOPE_VALIDATION: Lazy<Regex> = Lazy::new(|| Regex::new("^[a-z\\-.:]+$").unwrap());

impl FromStr for Scope {
    type Err = InvalidScopeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(" ") {
            return Err(InvalidScopeFromStrError);
        }
        if let Some(internal) = InternalScope::from_str(s) {
            return Ok(Self::Internal(internal));
        }
        if let Some(m) = SCOPE_VALIDATION.find(s) {
            return Ok(Self::Custom(m.as_str().into()));
        } else {
            return Err(InvalidScopeFromStrError);
        }
    }
}

#[derive(Debug, Display, Serialize)]
#[display("{self:?}")]
pub struct OAuthErrorData {
    #[serde(rename = "error")]
    kind: OAuthErrorKind,
    description: Option<String>,
    state: Option<String>,
}
pub struct OAuthQuery<T>(T);

#[axum::async_trait]
impl<T: DeserializeOwned, S: Send + Sync> FromRequestParts<S> for OAuthQuery<T> {
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let extractor = Query::from_request_parts(parts, state)
            .await
            .map_err(|err| NewError::invalid_request(Some(err.body_text()), None, None, None))?;
        Ok(Self(extractor.0))
    }
}

pub async fn authorize_request(
    State(state): State<AppState>,
    ApiAuth(auth): ApiAuth,
    method: Method,
    OAuthQuery(parameters): OAuthQuery<OAuthAuthorizeParameters>,
) -> AppResult<Response> {
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select * from applications where client_id = $1")
        .await?;
    let application = conn
        .query_opt(&stmt, &[&parameters.client_id])
        .await?
        .ok_or_else(|| NewError::invalid_client(None, None, None).into_error())?;
    let uri = Url::from_str(&parameters.redirect_uri)
        .map_err(|_| NewError::invalid_redirect_uri(None, None, None).into_error())?;
    {
        let uris: Vec<String> = application.get("redirect_uri");
        if !uris.contains(&parameters.redirect_uri) {
            return Err(NewError::invalid_redirect_uri(None, None, None).into());
        }
    }
    // let client_secret: Option<String> = application.get("client_secret");
    // if let Some(client_secret) = client_secret {
    //     if let Some(secret) = parameters.client_secret {
    //         if secret != client_secret {
    //             return Err(OAuthError::InvalidClientSecret.into());
    //         }
    //     } else {
    //         return Err(OAuthError::InvalidClientSecret.into());
    //     }
    // }
    match parameters.response_type {
        ResponseType::Code => {}
        _ => {
            return Err(NewError::authorize_unsupported_response_type(
                None,
                parameters.state,
                None,
                Some(uri),
            )
            .into())
        }
    }
    match parameters.response_mode {
        ResponseMode::Query => {}
        // ResponseMode::Fragment => {}
        _ => return Err(NewError::invalid_request(None, parameters.state, None, Some(uri)).into()),
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
    )
    .map_err(|_| {
        NewError::invalid_scope(None, parameters.state.clone(), None, Some(uri.clone()))
    })?;
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
            )
            .map_err(|_| {
                NewError::invalid_scope(None, parameters.state.clone(), None, Some(uri.clone()))
            })?;
            let denied = parameters.other.contains_key("denied");
            if denied {
                return Err(NewError::authorize_access_denied(
                    None,
                    parameters.state,
                    None,
                    Some(uri),
                )
                .into());
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
) -> Result<(Vec<Scope>, Vec<String>), InvalidScopeFromStrError> {
    let scope_errors = Vec::new();
    let scopes: Result<Vec<Scope>, _> = scopes
        .map(|s| Scope::from_str(s.as_str()))
        // .filter_map(|v| match v.1 {
        //     Ok(v) => Some(v),
        //     Err(_err) => {
        //         scope_errors.push(v.0.clone());
        //         None
        //     }
        // })
        .collect();
    let mut scopes = scopes?;
    let mut forbidden_scopes: Vec<Scope> = Vec::new();
    for scope in &scopes {
        if let Scope::Internal(internal) = scope {
            if !allowed_scopes.contains(internal) {
                forbidden_scopes.push(scope.clone());
            }
        }
    }
    scopes.retain(|s| !forbidden_scopes.contains(s));
    Ok((scopes, scope_errors))
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
