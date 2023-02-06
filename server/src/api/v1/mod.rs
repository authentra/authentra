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
