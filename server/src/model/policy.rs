use serde::{Deserialize, Serialize};
use time::ext::NumericalDuration;

use crate::executor::ExecutionContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub uid: i32,
    pub slug: String,
    pub kind: PolicyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyKind {
    PasswordExpiry { max_age: i64 },
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

impl PolicyKind {
    pub fn validate(&self, context: &ExecutionContext) -> PolicyResult {
        match self {
            PolicyKind::PasswordExpiry { max_age } => context
                .user
                .as_ref()
                .map(|user| user.password_change_date)
                .map_or(PolicyResult::NotApplicable, |date| {
                    (&(context.start_time - date) > &max_age.seconds()).into()
                }),
            PolicyKind::PasswordStrength => todo!(),
            PolicyKind::Expression => todo!(),
        }
    }
}
