use async_trait::async_trait;

use model::{FlowComponent, PasswordComponentData, Sources, Stage, StageKind};

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
                password_stage,
                user_fields,
            } => {
                let _stage = match password_stage {
                    Some(v) => Some(execution.lookup_stage(*v).await),
                    None => None,
                };
                Some(FlowComponent::Identification {
                    user_fields: user_fields.to_owned(),
                    sources: Sources {
                        sources: vec![],
                        show_source_labels: false,
                    },
                    password: (*password_stage).map(|_| PasswordComponentData {
                        recovery_url: "".into(),
                    }),
                })
            }
            StageKind::UserLogin | StageKind::UserLogout | StageKind::UserWrite => {
                tracing::warn!("Tried to send server side stage to client");
                None
            }
            StageKind::Password { .. } => Some(FlowComponent::Password {
                data: PasswordComponentData {
                    recovery_url: "".into(),
                },
            }),
            StageKind::Consent { mode: _ } => todo!(),
        }
    }
}
