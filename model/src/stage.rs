use serde::{Deserialize, Serialize};

use super::{PromptBinding, Reference};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Stage {
    pub uid: i32,
    pub slug: String,
    pub kind: StageKind,
    pub timeout: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PasswordBackend {
    Internal,
    LDAP,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "user_field", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum UserField {
    Email,
    Name,
    Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageKind {
    Deny,
    Prompt {
        bindings: Vec<PromptBinding>,
    },
    Identification {
        password: Option<Reference<Stage>>,
        user_fields: Vec<UserField>,
    },
    UserLogin,
    UserLogout,
    UserWrite,
    Password {
        backends: Vec<PasswordBackend>,
    },
    Consent {
        mode: ConsentMode,
    },
}

impl StageKind {
    pub const fn requires_input(&self) -> bool {
        match self {
            StageKind::Deny => true,
            StageKind::Prompt { .. } => true,
            StageKind::Identification { .. } => true,
            StageKind::UserLogin | StageKind::UserLogout | StageKind::UserWrite => false,
            StageKind::Password { .. } => true,
            StageKind::Consent { .. } => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ConsentMode {
    Always,
    Once,
    Until { duration: i32 },
}
