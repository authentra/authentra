use datacache::DataRef;
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

use super::PromptBinding;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
pub struct Stage {
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub uid: i32,
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub slug: String,
    pub kind: StageKind,
    pub timeout: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromSql, ToSql)]
#[serde(rename_all = "snake_case")]
#[postgres(name = "password_backend")]
pub enum PasswordBackend {
    #[postgres(name = "internal")]
    Internal,
    #[postgres(name = "ldap")]
    LDAP,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, FromSql, ToSql)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "user_field", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
#[postgres(name = "user_field")]
pub enum UserField {
    #[postgres(name = "email")]
    Email,
    #[postgres(name = "name")]
    Name,
    #[postgres(name = "uuid")]
    Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageKind {
    Deny,
    Prompt {
        bindings: Vec<PromptBinding>,
    },
    Identification {
        password: Option<DataRef<Stage>>,
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

#[derive(Debug, Clone, PartialEq, FromSql, ToSql)]
#[postgres(name = "consent_mode")]
pub enum PgConsentMode {
    #[postgres(name = "always")]
    Always,
    #[postgres(name = "once")]
    Once,
    #[postgres(name = "until")]
    Until,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ConsentMode {
    Always,
    Once,
    Until { duration: i32 },
}
