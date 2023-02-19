use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PartialUser {
    pub uid: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}
