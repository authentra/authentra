use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Policy, Reference, Stage};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "authentication_requirement", rename_all = "snake_case")]
pub enum AuthenticationRequirement {
    Superuser,
    Required,
    None,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "flow_designation", rename_all = "snake_case")]
pub enum FlowDesignation {
    Authentication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct FlowBinding {
    pub enabled: bool,
    pub negate: bool,
    pub order: i16,
    pub kind: FlowBindingKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowBindingKind {
    Group(Uuid),
    User(Uuid),
    Policy(Reference<Policy>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEntry {
    pub ordering: i16,
    pub bindings: Vec<FlowBinding>,
    pub stage: Reference<Stage>,
}
