use serde::Serialize;

use crate::model::{Stage, StageKind};

use super::flow::FlowExecution;

#[derive(Serialize)]
pub struct FlowData {
    pub flow: FlowInfo,
    pub pending_user: Option<PendingUser>,
    #[serde(flatten)]
    pub component: FlowComponent,
}

#[derive(Serialize)]
#[serde(tag = "component", rename_all = "kebab-case")]
pub enum FlowComponent {
    AccessDenied {
        message: String,
    },
    Identification {
        user_fields: Vec<String>,
        #[serde(flatten)]
        sources: Sources,
    },
    Password {
        recovery_url: String,
    },
}

impl Stage {
    pub fn as_component(&self, _execution: &FlowExecution) -> Option<FlowComponent> {
        match &self.kind {
            StageKind::Deny => Some(FlowComponent::AccessDenied {
                message: "Access denied".to_owned(),
            }),
            StageKind::Prompt { bindings: _ } => todo!(),
            StageKind::Identification { password: _ } => todo!(),
            StageKind::UserLogin => todo!(),
            StageKind::UserLogout => todo!(),
            StageKind::UserWrite => todo!(),
            StageKind::Password { .. } => Some(FlowComponent::Password {
                recovery_url: "".to_owned(),
            }),
            StageKind::Consent { mode: _ } => todo!(),
        }
    }
}

#[derive(Serialize)]
pub struct Sources {
    pub sources: Vec<Source>,
    pub show_source_labels: bool,
}

#[derive(Serialize)]
pub struct PendingUser {
    pub name: String,
    pub avatar_url: String,
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
