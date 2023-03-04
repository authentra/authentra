use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

use super::Reference;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PromptBinding {
    pub order: i16,
    pub prompt: Reference<Prompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Prompt {
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub uid: i32,
    pub field_key: String,
    pub label: String,
    pub kind: PromptKind,
    pub placeholder: Option<String>,
    pub required: bool,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromSql, ToSql)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[postgres(name = "prompt_kind")]
pub enum PromptKind {
    #[postgres(name = "username")]
    Username,
    #[postgres(name = "email")]
    Email,
    #[postgres(name = "password")]
    Password,
    #[postgres(name = "text")]
    Text,
    #[postgres(name = "text_read_only")]
    TextReadOnly,
    #[postgres(name = "signed_number")]
    SignedNumber,
    #[postgres(name = "unsigned_number")]
    UnsignedNumber,
    #[postgres(name = "checkbox")]
    Checkbox,
    #[postgres(name = "switch")]
    Switch,
    #[postgres(name = "date")]
    Date,
    #[postgres(name = "date_time")]
    DateTime,
    #[postgres(name = "seperator")]
    Seperator,
    #[postgres(name = "static")]
    Static,
    #[postgres(name = "locale")]
    Locale,
}
