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

#[derive(
    Debug,
    Clone,
    Display,
    Serialize,
    Deserialize,
    FromSql,
    ToSql,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
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

impl InternalScope {
    pub const fn values() -> [InternalScope; 2] {
        [InternalScope::ProfileRead, InternalScope::ProfileWrite]
    }
    pub const fn string_values() -> [&'static str; 2] {
        ["profile:read", "profile:write"]
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "profile:read" => Some(Self::ProfileRead),
            "profile:write" => Some(Self::ProfileWrite),
            _ => None,
        }
    }
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
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1/users", user::router())
        .nest("/api/v1/admin", admin::router())
        .nest("/api/internal/oauth", oauth::router())
        .nest("/api/v1/applications", applications::router())
        .nest("/api/v1/application-groups", application_groups::router())
        .layer(middlewares)
}
#[instrument]
async fn hello_world() -> &'static str {
    "Hello, World!"
}
