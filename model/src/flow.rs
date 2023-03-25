use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSql, FromSql)]
#[postgres(name = "authentication_requirement")]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationRequirement {
    #[postgres(name = "superuser")]
    Superuser,
    #[postgres(name = "required")]
    Required,
    #[postgres(name = "none")]
    None,
    #[postgres(name = "ignored")]
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSql, FromSql)]
#[postgres(name = "flow_designation")]
#[serde(rename_all = "snake_case")]
pub enum FlowDesignation {
    #[postgres(name = "invalidation")]
    Invalidation,
    #[postgres(name = "authentication")]
    Authentication,
    #[postgres(name = "authorization")]
    Authorization,
    #[postgres(name = "enrollment")]
    Enrollment,
    #[postgres(name = "recovery")]
    Recovery,
    #[postgres(name = "unenrollment")]
    Unenrollment,
    #[postgres(name = "configuration")]
    Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
pub struct Flow {
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub uid: i32,
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub slug: String,
    pub title: String,
    pub designation: FlowDesignation,
    pub authentication: AuthenticationRequirement,
    pub bindings: Vec<FlowBinding>,
    pub entries: Vec<FlowEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
pub struct FlowBinding {
    pub enabled: bool,
    pub negate: bool,
    pub order: i16,
    pub kind: FlowBindingKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum FlowBindingKind {
    Group(Uuid),
    User(Uuid),
    Policy(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEntry {
    pub ordering: i16,
    pub bindings: Vec<FlowBinding>,
    pub stage: i32,
}

#[cfg(feature = "axum")]
#[derive(Deserialize)]
pub struct FlowParam {
    pub flow_slug: String,
}
