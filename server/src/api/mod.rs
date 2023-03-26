use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use axum::{
    extract::{
        rejection::{PathRejection, QueryRejection},
        FromRequestParts, Path, Query,
    },
    response::{IntoResponse, Response},
    Json,
};
use deadpool_postgres::PoolError;
use derive_more::{Display, Error, From};
use http::{header::HeaderName, request::Parts, StatusCode};

use jsonwebtoken::{errors::ErrorKind, DecodingKey, EncodingKey};
use model::{error::SubmissionError, Flow, FlowParam};
use serde::Deserialize;
use storage::{EntityId, StorageError};
use tokio_postgres::error::SqlState;
use tracing_error::SpanTrace;
pub use v1::setup_api_v1;

pub mod csrf;
mod v1;

pub static CSRF_HEADER: HeaderName = HeaderName::from_static("x-csrf-token");

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

pub mod errors {
    use super::{ApiError, ApiErrorKind};

    pub const NOT_FOUND: ApiError = ApiError {
        kind: ApiErrorKind::NotFound,
        st: None,
    };
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
    Axum(#[error(source)] axum::Error),

    InvalidLoginData,
    SessionCookieMissing,
    InvalidSessionCookie,
    NotFound,

    #[from(ignore)]
    #[display("{}", _0)]
    MiscInternal(#[error(not(source))] &'static str),
    Unauthorized,
    Forbidden,
    Conflict,

    #[display("Missing middleware: '{}'", _0)]
    #[from(ignore)]
    MissingMiddleware(#[error(not(source))] &'static str),

    #[from(ignore)]
    #[display("{}", _0)]
    PwHash(#[error(not(source))] argon2::password_hash::Error),
    PoolError(#[error(source)] PoolError),
    #[from(ignore)]
    PostgresError(#[error(source)] tokio_postgres::Error),

    #[from]
    SubmissionError(#[error(source)] SubmissionError),
    #[from(ignore)]
    JsonWebToken(#[error(source)] jsonwebtoken::errors::Error),

    #[from(ignore)]
    #[display("Missing entity! {:?}", _0)]
    InternalEntityNotFound(#[error(not(source))] EntityId),
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

impl From<tokio_postgres::Error> for ApiErrorKind {
    fn from(value: tokio_postgres::Error) -> Self {
        if let Some(db_error) = value.as_db_error() {
            if db_error.code() == &SqlState::UNIQUE_VIOLATION {
                return Self::Conflict;
            }
        }
        Self::PostgresError(value)
    }
}
impl From<StorageError> for ApiErrorKind {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Pool(err) => ApiErrorKind::PoolError(err),
            StorageError::Database(err) => ApiErrorKind::PostgresError(err),
        }
    }
}

impl ApiErrorKind {
    pub fn is_internal(&self) -> bool {
        match self {
            ApiErrorKind::InvalidLoginData => false,
            ApiErrorKind::PwHash(_) => true,
            ApiErrorKind::NotFound => false,
            ApiErrorKind::MiscInternal(_) => true,
            ApiErrorKind::SubmissionError(_) => false,
            ApiErrorKind::SessionCookieMissing => false,
            ApiErrorKind::InvalidSessionCookie => false,
            ApiErrorKind::MissingMiddleware(_) => true,
            ApiErrorKind::Unauthorized => false,
            ApiErrorKind::JsonWebToken(_) => true,
            ApiErrorKind::Forbidden => false,
            ApiErrorKind::Conflict => false,
            ApiErrorKind::PoolError(_) => true,
            ApiErrorKind::PostgresError(_) => true,
            ApiErrorKind::Axum(_) => true,
            ApiErrorKind::InternalEntityNotFound(_) => true,
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
            if let Some(trace) = &self.st {
                let format = format!("{trace}");
                tracing::error!(trace = format, info = %self.kind, "Internal Server error");
            } else {
                tracing::error!(info = %self.kind, "Internal Server error");
            }
        }
        match self.kind {
            ApiErrorKind::Axum(_)
            | ApiErrorKind::PwHash(_)
            | ApiErrorKind::PoolError(_)
            | ApiErrorKind::PostgresError(_)
            | ApiErrorKind::MiscInternal(_)
            | ApiErrorKind::MissingMiddleware(_)
            | ApiErrorKind::InternalEntityNotFound(_)
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
            ApiErrorKind::Forbidden => StatusCode::FORBIDDEN.into_response(),
            ApiErrorKind::Conflict => StatusCode::CONFLICT.into_response(),
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
pub struct InternalExecutorQuery {
    pub query: String,
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
        let query: Query<InternalExecutorQuery> = Query::from_request_parts(parts, state)
            .await
            .map_err(ExecutorQueryRejection::Chained)?;
        Ok(serde_urlencoded::from_str(query.0.query.as_str())
            .map_err(ExecutorQueryRejection::FailedToDeserializeQueryString)?)
    }
}

pub async fn ping_handler() -> &'static str {
    "Pong!"
}

pub struct SlugWrapper<D>(pub String, PhantomData<D>);

impl<D> SlugWrapper<D> {
    pub fn new(slug: String) -> Self {
        Self(slug, PhantomData)
    }
}

#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for SlugWrapper<Flow>
where
    S: Send + Sync,
{
    type Rejection = PathRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path: Path<FlowParam> = Path::from_request_parts(parts, state).await?;
        let flow_slug = path.0.flow_slug;
        Ok(SlugWrapper::new(flow_slug))
    }
}
