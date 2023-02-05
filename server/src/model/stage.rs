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
pub enum PasswordBackend {
    Internal,
    LDAP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageKind {
    Deny,
    Prompt { bindings: Vec<PromptBinding> },
    Identification { password: Option<Reference<Stage>> },
    UserLogin,
    UserLogout,
    UserWrite,
    Password { backends: Vec<PasswordBackend> },
    Consent { mode: ConsentMode },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentMode {
    Always,
    Once,
    Until { duration: i64 },
}
