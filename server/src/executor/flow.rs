use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use http::Uri;
use parking_lot::{lock_api::RwLockReadGuard, Mutex, RawRwLock, RwLock};
use policy_engine::{execute, uri::Scheme, LogEntry};
use storage::datacache::{Data, DataRef, LookupRef};
use uuid::Uuid;

use crate::{
    api::ExecutorQuery,
    service::policy::{create_scope, PolicyService},
};
use model::{
    error::SubmissionError, user::PartialUser, AuthenticationRequirement, Flow, FlowBinding,
    FlowBindingKind, FlowComponent, FlowData, FlowEntry, FlowInfo, PendingUser, Policy,
    PolicyQuery, Stage,
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

    pub fn get_check_context(&self, context: CheckContextRequest) -> CheckContext {
        let pending_user = self.get_context().pending.clone();
        let context = CheckContext {
            inner: CheckContextData {
                request: context,
                pending_user,
            },
            execution: self.clone(),
        };
        context
    }

    pub async fn data(&self, error: Option<SubmissionError>, context: &CheckContext) -> FlowData {
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

    pub async fn lookup_stage(&self, reference: &DataRef<Stage>) -> Data<Stage> {
        let lock = self.0.context.read();
        let storage = &lock.storage;
        match storage.lookup(reference).await {
            Some(v) => v,
            None => panic!("Missing stage in storage {reference:?}"),
        }
    }
    pub async fn lookup_policy(&self, reference: &DataRef<Policy>) -> Data<Policy> {
        let lock = self.0.context.read();
        let storage = &lock.storage;
        match storage.lookup(reference).await {
            Some(v) => v,
            None => panic!("Missing policy in storage {reference:?}"),
        }
    }

    pub async fn check(&self, context: &CheckContext) -> Result<Option<String>, ()> {
        let flow = &self.0.flow;
        let auth_check = FlowCheck::Authentication(flow.authentication.clone());
        if !*auth_check.check(context).await {
            return Ok(Some(auth_check.message(context)));
        }
        for binding in &flow.bindings {
            if let Some(message) = check_binding(&binding, context).await? {
                return Ok(Some(message));
            }
        }
        let entry = self.get_entry();
        for binding in &entry.bindings {
            if let Some(message) = check_binding(&binding, context).await? {
                return Ok(Some(message));
            }
        }
        Ok(None)
    }
}

async fn check_binding(
    binding: &FlowBinding,
    context: &CheckContext,
) -> Result<Option<String>, ()> {
    if !binding.enabled {
        return Ok(None);
    }
    let check = FlowCheck::from(binding.kind.clone());
    let result = check.check(context).await;
    let result = if binding.negate {
        result.negate()
    } else {
        result
    };
    let result = *result;
    if !result {
        Ok(Some(check.message(&context)))
    } else {
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
    Policy(DataRef<Policy>),
}

impl FlowCheck {
    pub async fn check(&self, context: &CheckContext) -> FlowCheckOutput {
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
            FlowCheck::Policy(policy) => {
                let policy = context.execution.lookup_policy(policy).await;
                check_policy(context, &policy).await
            }
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

async fn check_policy(context: &CheckContext, policy: &Policy) -> FlowCheckOutput {
    match &policy.kind {
        model::PolicyKind::PasswordExpiry { max_age } => {
            let duration = time::Duration::seconds(*max_age as i64);
            let start_time = context.execution.get_context().start_time.clone();
            context
                .request
                .user
                .as_ref()
                .map(|user| user.password_change_date)
                .map(|date| ((start_time - date) < duration).into_output())
                .unwrap_or(FlowCheckOutput::Neutral)
        }
        model::PolicyKind::PasswordStrength => FlowCheckOutput::Neutral,
        model::PolicyKind::Expression(_) => {
            let reference = DataRef::new(PolicyQuery::uid(policy.uid));
            let ast = context
                .execution
                .0
                .policy_service
                .get_ast(reference.clone())
                .await;
            if let Some(ast) = ast {
                let scope = create_scope(&context);
                let result = execute(ast.as_ref(), || scope);
                let passed = match result.result {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::warn!(policy = ?policy,"An error occurred while executing policy!, {err}");
                        dispatch_expression_log_entries(&reference, result.output, true);
                        return FlowCheckOutput::FailedHard;
                    }
                };
                let output = if passed {
                    FlowCheckOutput::Passed
                } else {
                    FlowCheckOutput::Failed
                };
                dispatch_expression_log_entries(&reference, result.output, false);
                output
            } else {
                tracing::warn!("Failed to find ast for policy {reference:?}");
                FlowCheckOutput::Neutral
            }
        }
    }
}

fn dispatch_expression_log_entries(
    policy: &DataRef<Policy>,
    entries: Vec<LogEntry>,
    has_error: bool,
) {
    for entry in entries {
        match entry {
            LogEntry::Text(text) => {
                if has_error {
                    tracing::warn!(policy = ?policy, "INFO: {text}");
                } else {
                    tracing::debug!(policy = ?policy, "INFO: {text}");
                }
            }
            LogEntry::Debug(entry) => {
                if has_error {
                    tracing::warn!(policy = ?policy, position = ?entry.position, source = ?entry.source, "DEBUG: {}", entry.text);
                } else {
                    tracing::debug!(policy = ?policy, position = ?entry.position, source = ?entry.source, "DEBUG: {}", entry.text);
                }
            }
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
    pub host: String,
    pub scheme: Scheme,
    pub query: ExecutorQuery,
    pub user: Option<PartialUser>,
}

pub(super) struct FlowExecutionInternal {
    pub(super) flow: Data<Flow>,
    pub(super) key: FlowKey,
    pub(super) context: RwLock<ExecutionContext>,
    pub(super) current_entry_idx: Mutex<usize>,
    pub(super) is_completed: AtomicBool,
    pub(super) executor: FlowExecutor,
    pub(super) policy_service: PolicyService,
}
