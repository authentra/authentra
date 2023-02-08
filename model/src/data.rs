use serde::Serialize;
use uuid::Uuid;

use crate::{error::SubmissionError, UserField};

#[derive(Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
pub struct FlowData {
    pub flow: FlowInfo,
    #[serde(rename = "response_error")]
    pub error: Option<SubmissionError>,
    pub pending_user: Option<PendingUser>,
    #[serde(flatten)]
    pub component: FlowComponent,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
#[serde(tag = "type", content = "component", rename_all = "kebab-case")]
pub enum FlowComponent {
    AccessDenied {
        message: String,
    },
    Identification {
        user_fields: Vec<UserField>,
        #[serde(flatten)]
        sources: Sources,
    },
    Password {
        recovery_url: String,
    },
    Redirect {
        to: String,
    },
    Error {
        message: String,
    },
}

#[derive(Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
pub struct Sources {
    pub sources: Vec<Source>,
    pub show_source_labels: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
pub struct PendingUser {
    #[serde(skip)]
    pub uid: Uuid,
    pub name: String,
    pub avatar_url: String,
    #[serde(skip)]
    pub authenticated: bool,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
pub struct Source {
    pub name: String,
    pub icon_url: String,
}

#[derive(Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[typeshare::typeshare]
pub struct FlowInfo {
    pub title: String,
}
