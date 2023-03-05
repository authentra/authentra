use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialPolicy {
    pub uid: i32,
    pub slug: String,
    pub kind: PolicyKindSimple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
pub struct Policy {
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub uid: i32,
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub slug: String,
    pub kind: PolicyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyKind {
    PasswordExpiry { max_age: i32 },
    PasswordStrength,
    Expression(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSql, FromSql)]
#[postgres(name = "policy_kind")]
#[serde(rename_all = "snake_case")]
pub enum PolicyKindSimple {
    #[postgres(name = "password_expiry")]
    PasswordExpiry,
    #[postgres(name = "password_strength")]
    PasswordStrength,
    #[postgres(name = "expression")]
    Expression,
}

impl<'a> From<&'a PolicyKind> for PolicyKindSimple {
    fn from(value: &'a PolicyKind) -> Self {
        match value {
            PolicyKind::PasswordExpiry { max_age: _ } => Self::PasswordExpiry,
            PolicyKind::PasswordStrength => Self::PasswordStrength,
            PolicyKind::Expression(_) => Self::Expression,
        }
    }
}

impl<'a> From<&'a Policy> for PartialPolicy {
    fn from(value: &'a Policy) -> Self {
        Self {
            uid: value.uid,
            slug: value.slug.clone(),
            kind: (&value.kind).into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyResult {
    NotApplicable,
    Value(bool),
}

impl From<bool> for PolicyResult {
    fn from(value: bool) -> Self {
        Self::Value(value)
    }
}
