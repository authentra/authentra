use std::borrow::Cow;

use derive_more::{Display, Error, From};
use http::{
    header::{HeaderName, LOCATION},
    StatusCode,
};

use poem::{error::ResponseError, IntoResponse};
use tracing_error::SpanTrace;
pub use v1::setup_api_v1;

use self::sql_tx::TxError;

pub mod csrf;
pub mod sql_tx;
mod v1;
pub const CSRF_HEADER: HeaderName = HeaderName::from_static("x-csrf-token");
#[derive(Debug, Display, Error)]
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
    NotFound,
    MiscInternal,

    #[from(ignore)]
    #[display("{}", 0)]
    PwHash(#[error(not(source))] argon2::password_hash::Error),
    Tx(#[error(source)] TxError),
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

pub struct ConvertedApiError {
    status: StatusCode,
}

impl ResponseError for ApiError {
    fn status(&self) -> http::StatusCode {
        match self.kind {
            ApiErrorKind::Database(_)
            | ApiErrorKind::PwHash(_)
            | ApiErrorKind::Tx(_)
            | ApiErrorKind::MiscInternal => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorKind::InvalidLoginData => StatusCode::UNAUTHORIZED,
            ApiErrorKind::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn as_response(&self) -> poem::Response
    where
        Self: std::error::Error + Send + Sync + 'static,
    {
        self.status().into_response()
    }
}

pub struct Redirect {
    status: StatusCode,
    uri: String,
}

impl Redirect {
    pub fn found(uri: &str) -> Self {
        Self {
            status: StatusCode::FOUND,
            uri: uri.into(),
        }
    }
    pub fn found_ownend(uri: String) -> Self {
        Self {
            status: StatusCode::FOUND,
            uri,
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> poem::Response {
        self.status.with_header(LOCATION, self.uri).into_response()
    }
}
