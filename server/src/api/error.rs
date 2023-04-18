use argon2::password_hash::Error as ArgonError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use deadpool_postgres::PoolError;
use derive_more::{Display, Error};
use tracing_error::SpanTrace;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    trace: Option<SpanTrace>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn trace(&self) -> Option<&SpanTrace> {
        self.trace.as_ref()
    }
}

#[derive(Debug, Display, Error)]
pub enum ErrorKind {
    Forbidden,
    PoolError(PoolError),
    PostgresError(tokio_postgres::Error),
    Argon(#[error(not(source))] ArgonError),
}

impl<T: Into<ErrorKind>> From<T> for Error {
    fn from(value: T) -> Self {
        let value = value.into();
        if value.is_internal() {
            let trace = SpanTrace::capture();
            Self {
                kind: value,
                trace: Some(trace),
            }
        } else {
            Self {
                kind: value,
                trace: None,
            }
        }
    }
}

fn argon_is_internal(err: &ArgonError) -> bool {
    match err {
        ArgonError::Password => false,
        _ => true,
    }
}

impl ErrorKind {
    fn status(&self) -> StatusCode {
        match self {
            ErrorKind::PoolError(_) | ErrorKind::PostgresError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ErrorKind::Forbidden => StatusCode::FORBIDDEN,
            ErrorKind::Argon(err) => match argon_is_internal(err) {
                true => StatusCode::INTERNAL_SERVER_ERROR,
                false => StatusCode::FORBIDDEN,
            },
        }
    }
    fn is_internal(&self) -> bool {
        match self {
            ErrorKind::PoolError(_) | ErrorKind::PostgresError(_) => true,
            ErrorKind::Forbidden => false,
            ErrorKind::Argon(err) => argon_is_internal(err),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut response = (self.kind.status(), "").into_response();
        response.extensions_mut().insert(self);
        response
    }
}
