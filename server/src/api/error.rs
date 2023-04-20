use std::{borrow::Cow, fmt::Debug};

use argon2::password_hash::Error as ArgonError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use deadpool_postgres::PoolError;
use derive_more::{Display, Error, From};
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use tracing_error::SpanTrace;

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
}

#[derive(Debug, Display, Error, From)]
pub enum ErrorKind {
    #[display("403 Forbidden")]
    Forbidden,
    #[display("Deadpool: {}", _0)]
    PoolError(PoolError),
    #[display("Postgres: {}", _0)]
    PostgresError(tokio_postgres::Error),
    #[display("Invalid Authorization header")]
    InvalidAuthorizationHeader,
    #[display("JWT: {}", _0)]
    Jwt(JwtError),
    #[display("Argon: {}", _0)]
    Argon(#[error(not(source))] ArgonError),
    #[display("Header '{header_name}' contains non ascii chars")]
    HeaderNotAscii { header_name: &'static str },
}

impl<T: Into<ErrorKind>> From<T> for Error {
    fn from(value: T) -> Self {
        let value = value.into();
        let response_error = value.response();
        let status = response_error.status;
        if true {
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

#[derive(Clone)]
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
        (self.status, cow).into_response()
    }
}

impl ErrorKind {
    fn response(&self) -> ResponseError {
        match self {
            ErrorKind::PoolError(_) | ErrorKind::PostgresError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into()
            }
            ErrorKind::Forbidden => StatusCode::FORBIDDEN.into(),
            ErrorKind::Jwt(err) => jwt_response(err),
            ErrorKind::Argon(err) => argon_error(err),
            ErrorKind::HeaderNotAscii { header_name } => (
                StatusCode::BAD_REQUEST,
                format!("Header '{header_name}' contains non-ascii chars"),
            )
                .into(),
            ErrorKind::InvalidAuthorizationHeader => {
                (StatusCode::UNAUTHORIZED, "Invalid Authorization header").into()
            }
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
