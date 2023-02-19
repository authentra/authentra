use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use http::Uri;
use parking_lot::{lock_api::RwLockReadGuard, Mutex, RawRwLock, RwLock};
use uuid::Uuid;

use crate::{api::ExecutorQuery, flow_storage::ReferenceLookup};
use model::{
    error::SubmissionError, user::PartialUser, AuthenticationRequirement, Flow, FlowBindingKind,
    FlowComponent, FlowData, FlowEntry, FlowInfo, PendingUser, Policy, Reference, Stage,
};

use super::{data::AsComponent, ExecutionContext, ExecutionError, FlowExecutor, FlowKey};

#[derive(Clone)]
pub struct FlowExecution(pub(super) Arc<FlowExecutionInternal>);

impl FlowExecution {
    pub fn get_context(&self) -> RwLockReadGuard<RawRwLock, ExecutionContext> {
        self.0.context.read()
    }
    pub fn use_mut_context<F: FnOnce(&mut ExecutionContext)>(&self, func: F) {
        let mut lock = self.0.context.try_write().expect("Context is locked");
        func(&mut *lock);
    }
    pub fn set_error(&self, error: ExecutionError) {
        self.use_mut_context(|ctx| ctx.error = Some(error));
    }

    pub async fn data(
        &self,
        error: Option<SubmissionError>,
        context: CheckContextRequest,
    ) -> FlowData {
        let pending_user = self.get_context().pending.clone();
        let context = CheckContext {
            inner: CheckContextData {
                request: context,
                pending_user,
            },
            execution: self.clone(),
        };
        let flow = self.0.flow.as_ref();
        let entry = self.get_entry();
        let stage = self.lookup_stage(&entry.stage).await;
        let flow_info = FlowInfo {
            title: flow.title.clone(),
        };
        let is_completed = self.0.is_completed.load(Ordering::Relaxed);
        if let Some(_) = self.check(&context).await.expect("FlowCheck failed") {
            if !is_completed {
                return FlowData {
                    flow: flow_info,
                    error,
                    pending_user: None,
                    component: FlowComponent::AccessDenied {
                        message: "Flow does not apply to current user".into(),
                    },
                };
            }
        }
        let component = if is_completed {
            match &context.request.query.next {
                Some(to) => {
                    self.0.executor.invalidate_flow(&self.0.key);
                    FlowComponent::Redirect { to: to.clone() }
                }
                None => FlowComponent::Error {
                    message: "Missing Redirect".to_owned(),
                },
            }
        } else {
            match &self.get_context().error {
                Some(err) => FlowComponent::Error {
                    message: err.message.clone(),
                },
                None => match stage.as_component(self).await {
                    Some(v) => v,
                    None => FlowComponent::AccessDenied {
                        message: "Internal error while constructing component".to_owned(),
                    },
                },
            }
        };
        FlowData {
            flow: flow_info,
            component,
            pending_user: self.0.context.read().pending.clone(),
            error,
        }
    }
    pub fn get_entry(&self) -> &FlowEntry {
        let entry_idx = self.0.current_entry_idx.lock().to_owned();
        let Some(entry) =self.0.flow.entries.get(entry_idx) else { panic!("Entry index out of bounds") };
        entry
    }

    pub fn complete_current(&self) {
        let mut lock = self.0.current_entry_idx.lock();
        let new = *lock + 1;
        if self.0.flow.entries.get(new).is_some() {
            *lock = new;
        } else {
            self.0.is_completed.store(true, Ordering::Relaxed);
        }
    }

    pub fn is_completed(&self) -> bool {
        self.0.is_completed.load(Ordering::Relaxed)
    }

    pub async fn lookup_stage(&self, reference: &Reference<Stage>) -> Arc<Stage> {
        let lock = self.0.context.read();
        let storage = &lock.storage;
        match storage.lookup_reference(reference).await {
            Some(v) => v,
            None => panic!("Missing stage in storage {reference:?}"),
        }
    }

    pub async fn check(&self, context: &CheckContext) -> Result<Option<String>, ()> {
        let flow = &self.0.flow;
        let auth_check = FlowCheck::Authentication(flow.authentication.clone());
        if !*auth_check.check(context) {
            return Ok(Some(auth_check.message(context)));
        }
        let entry = self.get_entry();
        for binding in &entry.bindings {
            let check = FlowCheck::from(binding.kind.clone());
            let result = check.check(context);
            let result = if binding.negate {
                result.negate()
            } else {
                result
            };
            if !*result {
                return Ok(Some(check.message(&context)));
            }
        }
        Ok(None)
    }
}

pub enum FlowCheckOutput {
    Passed,
    Failed,
    FailedHard,
    Neutral,
}

impl FlowCheckOutput {
    pub fn negate(self) -> Self {
        match self {
            FlowCheckOutput::Passed => Self::Failed,
            FlowCheckOutput::Failed => Self::Passed,
            FlowCheckOutput::FailedHard => Self::FailedHard,
            FlowCheckOutput::Neutral => Self::Neutral,
        }
    }
}

impl Deref for FlowCheckOutput {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        match &self {
            FlowCheckOutput::Passed | FlowCheckOutput::Neutral => &true,
            FlowCheckOutput::Failed | FlowCheckOutput::FailedHard => &false,
        }
    }
}

pub trait IntoFlowCheckOutput {
    fn into_output(self) -> FlowCheckOutput;
}

impl IntoFlowCheckOutput for bool {
    fn into_output(self) -> FlowCheckOutput {
        if self {
            FlowCheckOutput::Passed
        } else {
            FlowCheckOutput::Failed
        }
    }
}

pub enum FlowCheck {
    Authentication(AuthenticationRequirement),
    IsUser(Uuid),
    Policy(Reference<Policy>),
}

impl FlowCheck {
    pub fn check(&self, context: &CheckContext) -> FlowCheckOutput {
        match self {
            FlowCheck::Authentication(requirement) => match requirement {
                AuthenticationRequirement::Superuser => {
                    if let Some(user) = &context.request.user {
                        user.is_admin.into_output()
                    } else {
                        FlowCheckOutput::Failed
                    }
                }
                AuthenticationRequirement::Required => context.request.user.is_some().into_output(),
                AuthenticationRequirement::None => context.request.user.is_none().into_output(),
                AuthenticationRequirement::Ignored => FlowCheckOutput::Neutral,
            },
            FlowCheck::IsUser(id) => {
                (context.request.user.as_ref().map(|v| &v.uid) == Some(id)).into_output()
            }
            FlowCheck::Policy(_) => todo!(),
        }
    }

    pub fn message(&self, _context: &CheckContext) -> String {
        match self {
            FlowCheck::Authentication(requirement) => match requirement {
                AuthenticationRequirement::Superuser => {
                    "You must be a superuser to access this flow"
                }
                AuthenticationRequirement::Required => "You must be logged in to access this flow",
                AuthenticationRequirement::None => "You must not be logged in to access this flow",
                AuthenticationRequirement::Ignored => "This is an error! ",
            }
            .into(),
            FlowCheck::IsUser(_) => "Access denied".into(),
            FlowCheck::Policy(_) => "Access denied".into(),
        }
    }

    pub fn negated_message(&self, context: &CheckContext) -> String {
        match self {
            FlowCheck::Authentication(_) => self.message(context),
            FlowCheck::IsUser(_) => self.message(context),
            FlowCheck::Policy(_) => self.message(context),
        }
    }
}

impl From<FlowBindingKind> for FlowCheck {
    fn from(value: FlowBindingKind) -> Self {
        match value {
            FlowBindingKind::Group(_) => todo!(),
            FlowBindingKind::User(id) => FlowCheck::IsUser(id),
            FlowBindingKind::Policy(policy) => FlowCheck::Policy(policy),
        }
    }
}

pub struct CheckContext {
    inner: CheckContextData,
    pub execution: FlowExecution,
}

impl Deref for CheckContext {
    type Target = CheckContextData;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct CheckContextData {
    pub request: CheckContextRequest,
    pub pending_user: Option<PendingUser>,
}

pub struct CheckContextRequest {
    pub uri: Uri,
    pub query: ExecutorQuery,
    pub user: Option<PartialUser>,
}

pub(super) struct FlowExecutionInternal {
    pub(super) flow: Arc<Flow>,
    pub(super) key: FlowKey,
    pub(super) context: RwLock<ExecutionContext>,
    pub(super) current_entry_idx: Mutex<usize>,
    pub(super) is_completed: AtomicBool,
    pub(super) executor: FlowExecutor,
}
