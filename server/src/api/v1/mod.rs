use std::backtrace::Backtrace;

use derive_more::{Display, Error, From};
use http::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey};

use poem::{
    error::ResponseError,
    get, handler,
    middleware::{AddData, CookieJarManager},
    EndpointExt, IntoResponse, Route,
};
use sqlx::PgPool;
use tracing::Span;
use tracing_error::SpanTrace;

use crate::api::{
    csrf::CsrfLayer,
    sql_tx::TxLayer,
    v1::{
        auth::{setup_auth_router, AuthServiceData},
        flow::setup_flow_router,
    },
};

use self::auth::AuthLayer;

use super::sql_tx::TxError;

pub mod application;
pub mod auth;
pub mod executor;
pub mod flow;

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
            ApiErrorKind::Database(_) | ApiErrorKind::PwHash(_) | ApiErrorKind::Tx(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
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

pub async fn setup_api_v1(
    secret: &str,
    pool: PgPool,
) -> poem::middleware::CookieJarManagerEndpoint<
    super::csrf::CsrfEndpoint<
        super::sql_tx::TxService<
            auth::AuthEndpoint<poem::middleware::AddDataEndpoint<Route, AuthServiceData>>,
            sqlx::Postgres,
        >,
    >,
> {
    let auth_data = AuthServiceData {
        encoding_key: EncodingKey::from_secret(secret.as_bytes()),
        decoding_key: DecodingKey::from_secret(secret.as_bytes()),
    };
    let router = Route::new()
        .at("/ping", get(ping_handler))
        .nest("/flow", setup_flow_router())
        .nest("/auth", setup_auth_router())
        .with(AddData::new(auth_data.clone()))
        .with(AuthLayer::new(auth_data))
        .with(TxLayer::new(pool))
        .with(CsrfLayer::new(vec!["*".into()]))
        .with(CookieJarManager::new());
    router
}

#[handler]
async fn ping_handler() -> &'static str {
    "Pong!"
}
