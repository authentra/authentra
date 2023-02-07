use serde::{Deserialize, Serialize};

use super::{PromptBinding, Reference};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub uid: i32,
    pub slug: String,
    pub kind: StageKind,
    pub timeout: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasswordBackend {
    Internal,
    LDAP,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_field", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserField {
    Email,
    Name,
    Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentMode {
    Always,
    Once,
    Until { duration: i64 },
}
