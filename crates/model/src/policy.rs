use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
pub struct Policy {
    pub uid: i32,
    pub slug: String,
    pub kind: PolicyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
#[serde(tag = "type", content = "policy", rename_all = "snake_case")]
pub enum PolicyKind {
    PasswordExpiry { max_age: i32 },
    PasswordStrength,
    Expression,
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

// impl PolicyKind {
//     pub fn validate(&self, context: &ExecutionContext) -> PolicyResult {
//         match self {
//             PolicyKind::PasswordExpiry { max_age } => context
//                 .user
//                 .as_ref()
//                 .map(|user| user.password_change_date)
//                 .map_or(PolicyResult::NotApplicable, |date| {
//                     (&(context.start_time - date) > &max_age.seconds()).into()
//                 }),
//             PolicyKind::PasswordStrength => todo!(),
//             PolicyKind::Expression => todo!(),
//         }
//     }
// }
