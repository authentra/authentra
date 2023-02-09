use std::fmt::Debug;

use async_trait::async_trait;
use axum::{
    extract::{rejection::QueryRejection, FromRequestParts, Query},
    response::{IntoResponse, Response},
    Json,
};
use derive_more::{Display, Error, From};
use http::{header::HeaderName, request::Parts, StatusCode};

use jsonwebtoken::{errors::ErrorKind, DecodingKey, EncodingKey};
use model::error::SubmissionError;
use serde::Deserialize;
use tracing_error::SpanTrace;
pub use v1::setup_api_v1;

use self::sql_tx::TxError;

pub mod csrf;
pub mod sql_tx;
mod v1;
pub const CSRF_HEADER: HeaderName = HeaderName::from_static("x-csrf-token");
#[derive(Debug, Display, Error)]
#[display("{} \n Trace: \n {:?}", self.kind, match &self.st {
    Some(v) => format!("{v}"),
    None => String::new()
})]
pub struct ApiError {
    pub kind: ApiErrorKind,
    pub st: Option<SpanTrace>,
    // #[error(backtrace)]
    // pub backtrace: Backtrace,
}

impl<T: Into<ApiErrorKind>> From<T> for ApiError {
    fn from(value: T) -> Self {
        let value = value.into();
        if value.is_internal() {
            let trace = SpanTrace::capture();
            Self {
                kind: value,
                st: Some(trace),
            }
        } else {
            Self {
                kind: value,
                st: None,
            }
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum ApiErrorKind {
    Database(#[error(source, backtrace)] sqlx::Error),

    InvalidLoginData,
    SessionCookieMissing,
    InvalidSessionCookie,
    NotFound,
    MiscInternal,
    Unauthorized,

    #[display("Missing middleware: '{}'", 0)]
    MissingMiddleware(#[error(not(source))] &'static str),

    #[from(ignore)]
    #[display("{}", 0)]
    PwHash(#[error(not(source))] argon2::password_hash::Error),
    Tx(#[error(source)] TxError),

    #[from]
    SubmissionError(#[error(source)] SubmissionError),
    #[from(ignore)]
    JsonWebToken(#[error(source)] jsonwebtoken::errors::Error),
}

impl From<jsonwebtoken::errors::Error> for ApiErrorKind {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        match value.kind() {
            ErrorKind::InvalidEcdsaKey => ApiErrorKind::JsonWebToken(value),
            ErrorKind::InvalidRsaKey(_) => ApiErrorKind::JsonWebToken(value),
            ErrorKind::RsaFailedSigning => ApiErrorKind::JsonWebToken(value),
            ErrorKind::InvalidAlgorithmName => ApiErrorKind::JsonWebToken(value),
            ErrorKind::InvalidKeyFormat => ApiErrorKind::JsonWebToken(value),
            ErrorKind::InvalidAlgorithm => ApiErrorKind::JsonWebToken(value),
            ErrorKind::MissingAlgorithm => ApiErrorKind::JsonWebToken(value),
            ErrorKind::Crypto(_) => ApiErrorKind::JsonWebToken(value),
            _ => ApiErrorKind::InvalidSessionCookie,
        }
    }
}

impl ApiErrorKind {
    pub fn is_internal(&self) -> bool {
        match self {
            ApiErrorKind::Database(_) => true,
            ApiErrorKind::InvalidLoginData => false,
            ApiErrorKind::PwHash(_) => true,
            ApiErrorKind::Tx(_) => true,
            ApiErrorKind::NotFound => false,
            ApiErrorKind::MiscInternal => true,
            ApiErrorKind::SubmissionError(_) => false,
            ApiErrorKind::SessionCookieMissing => false,
            ApiErrorKind::InvalidSessionCookie => false,
            ApiErrorKind::MissingMiddleware(_) => true,
            ApiErrorKind::Unauthorized => false,
            ApiErrorKind::JsonWebToken(_) => true,
        }
    }

    pub fn into_api(self) -> ApiError {
        ApiError::from(self)
    }
}

impl From<argon2::password_hash::Error> for ApiErrorKind {
    fn from(value: argon2::password_hash::Error) -> Self {
        match value {
            argon2::password_hash::Error::Password => ApiErrorKind::InvalidLoginData,
            err => ApiErrorKind::PwHash(err),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.kind.is_internal() {
            tracing::error!("{self}");
        }
        match self.kind {
            ApiErrorKind::Database(_)
            | ApiErrorKind::PwHash(_)
            | ApiErrorKind::Tx(_)
            | ApiErrorKind::MiscInternal
            | ApiErrorKind::MissingMiddleware(_)
            | ApiErrorKind::JsonWebToken(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            ApiErrorKind::InvalidLoginData => StatusCode::UNAUTHORIZED.into_response(),
            ApiErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
            ApiErrorKind::SubmissionError(err) => {
                let mut response = Json(err).into_response();
                *response.status_mut() = StatusCode::BAD_REQUEST;
                response
            }
            ApiErrorKind::SessionCookieMissing | ApiErrorKind::Unauthorized => {
                StatusCode::UNAUTHORIZED.into_response()
            }
            ApiErrorKind::InvalidSessionCookie => {
                (StatusCode::UNAUTHORIZED, "Invalid session cookie").into_response()
            }
        }
    }
}
#[derive(Clone)]
pub struct AuthServiceData {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
}
impl Debug for AuthServiceData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthServiceData").finish()
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ExecutorQuery {
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InternalQuery {
    query: String,
}

pub enum ExecutorQueryRejection {
    Chained(QueryRejection),
    FailedToDeserializeQueryString(serde_urlencoded::de::Error),
}

impl IntoResponse for ExecutorQueryRejection {
    fn into_response(self) -> Response {
        match self {
            ExecutorQueryRejection::Chained(v) => v.into_response(),
            ExecutorQueryRejection::FailedToDeserializeQueryString(_v) => {
                (StatusCode::BAD_REQUEST, "Failed to deserialize query").into_response()
            }
        }
    }
}
#[async_trait]
impl<S> FromRequestParts<S> for ExecutorQuery
where
    S: Send + Sync,
{
    type Rejection = ExecutorQueryRejection;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<ExecutorQuery, Self::Rejection> {
        let query: Query<InternalQuery> = Query::from_request_parts(parts, state)
            .await
            .map_err(|err| ExecutorQueryRejection::Chained(err))?;
        Ok(serde_urlencoded::from_str(query.0.query.as_str())
            .map_err(|err| ExecutorQueryRejection::FailedToDeserializeQueryString(err))?)
    }
}
