use derive_more::{Display, Error, From};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Null,
    Boolean,
    String,
    Number,
    Object,
    Array,
}

impl<'a> From<&'a Value> for FieldType {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Null => FieldType::Null,
            Value::Bool(_) => FieldType::Boolean,
            Value::Number(_) => FieldType::Number,
            Value::String(_) => FieldType::String,
            Value::Array(_) => FieldType::Array,
            Value::Object(_) => FieldType::Object,
        }
    }
}

#[derive(Debug, Clone, Error, Display, Serialize, From)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubmissionError {
    NoPendingUser,
    #[from]
    Field(#[error(source)] FieldError),
}

#[derive(Debug, Clone, Error, Display, Serialize)]
pub struct FieldError {
    #[serde(flatten)]
    kind: FieldErrorKind,
    field: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, kind: FieldErrorKind) -> Self {
        Self {
            field: field.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, Error, Display, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldErrorKind {
    Missing,
    Invalid { message: String },
    InvalidType { expected: FieldType, got: FieldType },
}

impl FieldErrorKind {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
        }
    }

    pub fn invalid_type(expected: FieldType, got: FieldType) -> Self {
        Self::InvalidType { expected, got }
    }
}
