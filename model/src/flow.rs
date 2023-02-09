#[cfg(feature = "axum")]
use axum::{
    extract::{rejection::PathRejection, Path},
    http::request::Parts,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Policy, Reference, Stage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "authentication_requirement", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationRequirement {
    Superuser,
    Required,
    None,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "flow_designation", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum FlowDesignation {
    Authentication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Flow {
    pub uid: i32,
    pub slug: String,
    pub title: String,
    pub designation: FlowDesignation,
    pub authentication: AuthenticationRequirement,
    pub bindings: Vec<FlowBinding>,
    pub entries: Vec<FlowEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FlowBinding {
    pub enabled: bool,
    pub negate: bool,
    pub order: i16,
    pub kind: FlowBindingKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged, rename_all = "snake_case")]
pub enum FlowBindingKind {
    Group(Uuid),
    User(Uuid),
    Policy(Reference<Policy>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FlowEntry {
    pub ordering: i16,
    pub bindings: Vec<FlowBinding>,
    pub stage: Reference<Stage>,
}

#[cfg(feature = "axum")]
#[derive(Deserialize)]
pub struct FlowParam {
    flow_slug: String,
}

#[cfg(feature = "axum")]
#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Reference<Flow>
where
    S: Send + Sync,
{
    type Rejection = PathRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path: Path<FlowParam> = Path::from_request_parts(parts, state).await?;
        let flow_slug = path.0.flow_slug;
        Ok(Reference::new_slug(flow_slug))
    }
}
