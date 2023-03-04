use serde::Serialize;
use uuid::Uuid;

use crate::{error::SubmissionError, UserField};

#[derive(Serialize)]
pub struct FlowData {
    pub flow: FlowInfo,
    #[serde(rename = "response_error")]
    pub error: Option<SubmissionError>,
    pub pending_user: Option<PendingUser>,
    #[serde(flatten)]
    pub component: FlowComponent,
}

#[derive(Serialize)]
#[serde(tag = "component", rename_all = "snake_case")]
pub enum FlowComponent {
    AccessDenied {
        message: String,
    },
    Identification {
        user_fields: Vec<UserField>,
        #[serde(flatten)]
        sources: Sources,
        password: Option<PasswordComponentData>,
    },
    Password {
        #[serde(flatten)]
        data: PasswordComponentData,
    },
    Redirect {
        to: String,
    },
    Error {
        message: String,
    },
}

#[derive(Serialize)]
pub struct PasswordComponentData {
    pub recovery_url: String,
}

#[derive(Serialize)]
pub struct Sources {
    pub sources: Vec<Source>,
    pub show_source_labels: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingUser {
    #[serde(skip)]
    pub uid: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
    #[serde(skip)]
    pub authenticated: bool,
    #[serde(skip)]
    pub is_admin: bool,
}

#[derive(Serialize)]
pub struct Source {
    pub name: String,
    pub icon_url: String,
}

#[derive(Serialize)]
pub struct FlowInfo {
    pub title: String,
}
