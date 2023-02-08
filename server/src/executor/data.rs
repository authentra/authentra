use async_trait::async_trait;

use model::{FlowComponent, Sources, Stage, StageKind};

use super::flow::FlowExecution;

#[async_trait]
pub trait AsComponent {
    async fn as_component(&self, execution: &FlowExecution) -> Option<FlowComponent>;
}

#[async_trait]
impl AsComponent for Stage {
    async fn as_component(&self, execution: &FlowExecution) -> Option<FlowComponent> {
        match &self.kind {
            StageKind::Deny => Some(FlowComponent::AccessDenied {
                message: "Access denied".to_owned(),
            }),
            StageKind::Prompt { bindings: _ } => todo!(),
            StageKind::Identification {
                password,
                user_fields,
            } => {
                let _stage = match password {
                    Some(v) => Some(execution.lookup_stage(&v).await),
                    None => None,
                };
                Some(FlowComponent::Identification {
                    user_fields: user_fields.to_owned(),
                    sources: Sources {
                        sources: vec![],
                        show_source_labels: false,
                    },
                })
            }
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
