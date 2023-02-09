use derive_more::{Display, Error, From};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Display, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubmissionError {
    MissingField {
        field_name: String,
    },
    InvalidFieldType {
        field_name: String,
        expected: FieldType,
        got: FieldType,
    },
    NoPendingUser,
    #[from]
    Field(#[error(source)] FieldError),
    Unauthenticated,
}
#[derive(Debug, Clone, Error, Display, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldError {
    Invalid { field: String, message: String },
}
