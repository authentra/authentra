use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use derive_more::Display;
use moka::sync::Cache;
use parking_lot::{Mutex, RwLock};

use crate::{
    auth::Session,
    flow_storage::{FlowStorage, FreezedStorage, ReferenceLookup, ReverseLookup},
    service::policy::PolicyService,
};
use model::{Flow, PolicyKind, PolicyResult, Reference, ReferenceKind};

use self::flow::{FlowExecution, FlowExecutionInternal};

mod context;
pub mod data;
pub mod flow;
pub mod storage;
pub use context::*;

// 12 Hours
const TIME_TO_IDLE: Duration = Duration::from_secs(60 * 60 * 12);
// 36 Hours
const TIME_TO_LIVE: Duration = Duration::from_secs(60 * 60 * 36);

#[derive(Debug, Display, Clone, Hash, PartialEq, Eq)]
pub struct FlowKey {
    session: String,
    flow: Reference<Flow>,
}

impl FlowKey {
    fn new(session: &Session, flow: Reference<Flow>) -> Self {
        Self {
            session: session.session_id.clone(),
            flow,
        }
    }
}

#[derive(Clone)]
pub struct FlowExecutor {
    internal: Arc<FlowExecutorInternal>,
}

struct FlowExecutorInternal {
    executions: Cache<FlowKey, FlowExecution>,
    storage: FlowStorage,
    policy_service: PolicyService,
}

impl FlowExecutorInternal {
    pub fn new(storage: FlowStorage, policy_service: PolicyService) -> Self {
        Self {
            executions: Cache::builder()
                .time_to_idle(TIME_TO_IDLE.clone())
                .time_to_live(TIME_TO_LIVE.clone())
                .build(),
            storage,
            policy_service,
        }
    }
}

impl FlowExecutor {
    pub fn new(storage: FlowStorage, policy_service: PolicyService) -> Self {
        Self {
            internal: Arc::new(FlowExecutorInternal::new(storage, policy_service)),
        }
    }

    pub fn invalidate_flow(&self, key: &FlowKey) {
        self.internal.executions.invalidate(key);
    }

    pub fn get_key(&self, session: &Session, flow: Reference<Flow>) -> Option<FlowKey> {
        if flow.kind() != ReferenceKind::Slug {
            return None;
        }
        let key = FlowKey::new(session, flow);
        Some(key)
    }

    pub async fn get_execution(&self, key: &FlowKey, start: bool) -> Option<FlowExecution> {
        let execution = self.internal.executions.get(key);
        if execution.is_some() {
            return execution;
        }

        if start {
            self.start(key).await
        } else {
            None
        }
    }

    pub async fn start(&self, key: &FlowKey) -> Option<FlowExecution> {
        let flow = match self.internal.storage.lookup_reference(&key.flow).await {
            Some(flow) => flow,
            None => {
                return None;
            }
        };
        let mut storage = FreezedStorage::new(self.internal.storage.clone());
        if flow.entries.is_empty() {
            return None;
        }
        flow.reverse_lookup(&storage).await;
        let errors = storage.freeze();
        if !errors.is_empty() {
            panic!("Unresolved references! {errors:?}");
        }
        let context = ExecutionContext::new(key.session.clone(), storage);
        let execution = FlowExecutionInternal {
            flow,
            context: RwLock::new(context),
            current_entry_idx: Mutex::new(0),
            is_completed: AtomicBool::new(false),
            executor: self.clone(),
            key: key.clone(),
            policy_service: self.internal.policy_service.clone(),
        };
        let execution = FlowExecution(Arc::new(execution));
        self.internal
            .executions
            .insert(key.clone(), execution.clone());
        Some(execution)
    }

    // pub fn init(&mut self) {}
}

pub trait Validate {
    type Res;

    fn validate(&self, context: &ExecutionContext) -> Self::Res;
}

impl Validate for PolicyKind {
    type Res = PolicyResult;
    fn validate(&self, context: &ExecutionContext) -> Self::Res {
        match self {
            PolicyKind::PasswordExpiry { max_age } => context
                .user
                .as_ref()
                .map(|user| user.password_change_date)
                .map_or(PolicyResult::NotApplicable, |date| {
                    (&(context.start_time - date) > &time::Duration::seconds(*max_age as i64))
                        .into()
                }),
            PolicyKind::PasswordStrength => todo!(),
            PolicyKind::Expression(..) => todo!(),
        }
    }
}
