use serde::{Deserialize, Serialize};

use super::Reference;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBinding {
    pub order: u16,
    pub prompt: Reference<Prompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub uid: i32,
    pub field_key: String,
    pub label: String,
    pub kind: PromptKind,
    pub placeholder: Option<String>,
    pub required: bool,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptKind {
    Username,
    Email,
    Password,
    Text,
    TextReadOnly,
    SignedNumber,
    UnsignedNumber,
    Checkbox,
    Switch,
    Date,
    DateTime,
    Seperator,
    Static,
    Locale,
}
