use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

use argon2::password_hash::Error as ArgonError;
use axum::{
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use deadpool_postgres::PoolError;
use derive_more::{Display, Error, From};
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use serde::Serialize;
use tokio::task::JoinError;
use tracing_error::SpanTrace;

use crate::{auth::AuthError, routes::oauth::OAuthError};

pub struct Error {
    kind: ErrorKind,
    response_error: ResponseError,
    trace: Option<SpanTrace>,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("trace", &self.trace)
            .finish()
    }
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn trace(&self) -> Option<&SpanTrace> {
        self.trace.as_ref()
    }
    pub fn format_trace(&self) -> Option<String> {
        self.trace
            .as_ref()
            .map(|trace| WrappedTrace(trace).to_string())
    }
}

struct WrappedTrace<'a>(&'a SpanTrace);

macro_rules! try_bool {
    ($e:expr, $dest:ident) => {{
        let ret = $e.unwrap_or_else(|e| $dest = Err(e));

        if $dest.is_err() {
            return false;
        }

        ret
    }};
}

impl<'a> Display for WrappedTrace<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut err = Ok(());
        self.0.with_spans(|metadata, _| {
            if let Some((file, line)) = metadata
                .file()
                .and_then(|file| metadata.line().map(|line| (file, line)))
            {
                try_bool!(write!(f, "\nat {}:{}", file, line), err);
            }
            true
        });
        err
    }
}

#[derive(Debug, Display)]
#[display("{status} '{message}'")]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl AsRef<str>) -> Self {
        Self {
            status,
            message: message.as_ref().into(),
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum ErrorKind {
    #[display("Status: {}", _0)]
    Status(#[error(not(source))] StatusCode),
    #[display("Api: {}", _0)]
    Api(#[error(not(source))] ApiError),
    #[display("Deadpool: {}", _0)]
    PoolError(PoolError),
    #[display("Postgres: {}", _0)]
    PostgresError(tokio_postgres::Error),
    #[display("{}", _0)]
    Auth(#[error(not(source))] AuthError),
    #[display("JWT: {}", _0)]
    Jwt(JwtError),
    #[display("Argon: {}", _0)]
    Argon(#[error(not(source))] ArgonError),
    #[display("Header '{header_name}' contains non ascii chars")]
    HeaderNotAscii { header_name: &'static str },
    #[display("Tokio failed to join task")]
    TokioJoin(JoinError),
    #[display("OAuth: {}", _0)]
    Oauth(#[error(not(source))] OAuthError),
    #[display("Json: {}", _0)]
    Json(JsonRejection),
    #[display("Query: {}", _0)]
    Query(QueryRejection),
}

impl ErrorKind {
    pub fn not_found() -> Self {
        Self::Status(StatusCode::NOT_FOUND)
    }
    pub fn forbidden() -> Self {
        Self::Status(StatusCode::FORBIDDEN)
    }
    pub fn internal() -> Self {
        Self::Status(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl<T: Into<ErrorKind>> From<T> for Error {
    fn from(value: T) -> Self {
        let value: ErrorKind = value.into();
        let response_error = value.response();
        let status = response_error.status;
        if status.is_server_error() {
            let trace = SpanTrace::capture();
            Self {
                kind: value,
                response_error,
                trace: Some(trace),
            }
        } else {
            Self {
                kind: value,
                response_error,
                trace: None,
            }
        }
    }
}

pub trait IntoError {
    fn into_error(self) -> Error;
}

impl<T: Into<ErrorKind>> IntoError for T {
    fn into_error(self) -> Error {
        Error::from(self)
    }
}

fn argon_error(err: &ArgonError) -> ResponseError {
    match err {
        ArgonError::Password => (StatusCode::UNAUTHORIZED, "Invalid password").into(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}

fn jwt_response(err: &JwtError) -> ResponseError {
    match err.kind() {
        JwtErrorKind::InvalidToken => (StatusCode::UNAUTHORIZED, "JWT: Malformed").into(),
        JwtErrorKind::InvalidSignature => {
            (StatusCode::UNAUTHORIZED, "JWT: Invalid signature").into()
        }
        JwtErrorKind::InvalidEcdsaKey
        | JwtErrorKind::InvalidRsaKey(_)
        | JwtErrorKind::RsaFailedSigning
        | JwtErrorKind::InvalidAlgorithmName
        | JwtErrorKind::MissingAlgorithm
        | JwtErrorKind::InvalidKeyFormat => StatusCode::INTERNAL_SERVER_ERROR.into(),
        JwtErrorKind::MissingRequiredClaim(_) => StatusCode::INTERNAL_SERVER_ERROR.into(),
        JwtErrorKind::ExpiredSignature => (StatusCode::UNAUTHORIZED, "JWT: Expired").into(),
        JwtErrorKind::InvalidIssuer => (StatusCode::UNAUTHORIZED, "JWT: Invalid issuer").into(),
        JwtErrorKind::InvalidAudience => (StatusCode::UNAUTHORIZED, "JWT: Invalid audience").into(),
        JwtErrorKind::InvalidSubject => (StatusCode::UNAUTHORIZED, "JWT: Invalid subject").into(),
        JwtErrorKind::ImmatureSignature => {
            (StatusCode::BAD_REQUEST, "JWT: Immature signature").into()
        }
        JwtErrorKind::InvalidAlgorithm => {
            (StatusCode::BAD_REQUEST, "JWT: Invalid algorithm").into()
        }
        JwtErrorKind::Base64(_) => (StatusCode::BAD_REQUEST, "JWT: Invalid").into(),
        JwtErrorKind::Json(_) => StatusCode::INTERNAL_SERVER_ERROR.into(),
        JwtErrorKind::Utf8(_) => StatusCode::INTERNAL_SERVER_ERROR.into(),
        JwtErrorKind::Crypto(_) => StatusCode::INTERNAL_SERVER_ERROR.into(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}

#[derive(Clone, Debug)]
pub struct ResponseError {
    status: StatusCode,
    message: Option<Cow<'static, str>>,
}

impl From<StatusCode> for ResponseError {
    fn from(status: StatusCode) -> Self {
        Self {
            status,
            message: None,
        }
    }
}
impl From<(StatusCode, &'static str)> for ResponseError {
    fn from(value: (StatusCode, &'static str)) -> Self {
        Self {
            status: value.0,
            message: Some(Cow::Borrowed(value.1)),
        }
    }
}
impl From<(StatusCode, String)> for ResponseError {
    fn from(value: (StatusCode, String)) -> Self {
        Self {
            status: value.0,
            message: Some(Cow::Owned(value.1)),
        }
    }
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response {
        let cow = self.message.unwrap_or(Cow::Borrowed(""));
        (
            self.status,
            Json(JsonErrorResponse {
                success: false,
                message: cow,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct JsonErrorResponse {
    success: bool,
    message: Cow<'static, str>,
}

impl ErrorKind {
    fn response(&self) -> ResponseError {
        match self {
            ErrorKind::PoolError(_) | ErrorKind::PostgresError(_) | ErrorKind::TokioJoin(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into()
            }
            ErrorKind::Status(status) => status.clone().into(),
            ErrorKind::Jwt(err) => jwt_response(err),
            ErrorKind::Argon(err) => argon_error(err),
            ErrorKind::HeaderNotAscii { header_name } => (
                StatusCode::BAD_REQUEST,
                format!("Header '{header_name}' contains non-ascii chars"),
            )
                .into(),
            ErrorKind::Auth(auth) => {
                let status = StatusCode::UNAUTHORIZED;
                match auth {
                    AuthError::MissingCookie => (status, "Authentication cookie missing").into(),
                    AuthError::InvalidSession => (status, "Invalid session").into(),
                    AuthError::MissingHeader => (status, "Authorization header missing").into(),
                    AuthError::InvalidHeader => (status, "Authorization header is invalid").into(),
                    AuthError::InvalidCredentials => (status, "Invalid credentials").into(),
                    AuthError::ClaimsMissingInInfo => (StatusCode::INTERNAL_SERVER_ERROR).into(),
                }
            }
            ErrorKind::Api(err) => (err.status, err.message.clone()).into(),
            ErrorKind::Oauth(_) => todo!(),
            ErrorKind::Json(json) => (json.status(), json.body_text()).into(),
            ErrorKind::Query(query) => (query.status(), query.body_text()).into(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut response = self.response_error.clone().into_response();
        response.extensions_mut().insert(self);
        response
    }
}
