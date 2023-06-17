use std::str::FromStr;

use axum::Router;
use derive_more::Display;
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tracing::instrument;

use crate::AppState;
mod admin;
mod application_groups;
mod applications;
mod auth;
pub mod oauth;
mod user;

#[derive(Debug, Display, Serialize, Deserialize, FromSql, ToSql, PartialEq, Eq)]
#[postgres(name = "internal_scopes")]
pub enum InternalScope {
    #[postgres(name = "profile:read")]
    #[serde(rename = "profile:read")]
    #[display("profile:read")]
    ProfileRead,
    #[postgres(name = "profile:write")]
    #[serde(rename = "profile:write")]
    #[display("profile:write")]
    ProfileWrite,
}

#[derive(Debug, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "application_kind")]
pub enum ApplicationKind {
    #[postgres(name = "web-server")]
    #[serde(rename = "web-server")]
    WebServer,
    #[postgres(name = "spa")]
    #[serde(rename = "spa")]
    SPA,
}

#[derive(Debug, Serialize, Deserialize, FromSql, ToSql)]
#[postgres(name = "consent_mode")]
#[serde(rename_all = "lowercase")]
pub enum ConsentMode {
    #[postgres(name = "web-server")]
    Explicit,
    #[postgres(name = "spa")]
    Implicit,
}

pub struct UnknownInternalScope;

impl FromStr for InternalScope {
    type Err = UnknownInternalScope;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "profile:read" => Ok(Self::ProfileRead),
            "profile:write" => Ok(Self::ProfileWrite),
            _ => Err(UnknownInternalScope),
        }
    }
}

pub fn setup_router() -> Router<AppState> {
    let middlewares = ServiceBuilder::new().layer(crate::telemetry::middleware::new());
    Router::new()
        .nest("/api/auth", auth::router())
        .nest("/api/users", user::router())
        .nest("/api/admin", admin::router())
        .nest("/oauth/authorize", oauth::router())
        .nest("/api/applications", applications::router())
        .nest("/api/application-groups", application_groups::router())
        .layer(middlewares)
}
#[instrument]
async fn hello_world() -> &'static str {
    "Hello, World!"
}
