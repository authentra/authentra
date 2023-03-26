use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use ::storage::ExecutorStorage;
use derive_more::Display;
use moka::sync::Cache;
use tokio::sync::RwLock;

use crate::{api::ApiError, auth::Session, service::policy::PolicyService};
use model::{PolicyKind, PolicyResult};

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
#[display("FlowKey({session}, {flow:?})")]
pub struct FlowKey {
    session: String,
    flow: i32,
}

impl FlowKey {
    fn new(session: &Session, flow: i32) -> Self {
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
    storage: Arc<dyn ExecutorStorage>,
    policy_service: PolicyService,
}

impl FlowExecutorInternal {
    pub fn new(storage: Arc<dyn ExecutorStorage>, policy_service: PolicyService) -> Self {
        Self {
            executions: Cache::builder()
                .time_to_idle(TIME_TO_IDLE)
                .time_to_live(TIME_TO_LIVE)
                .build(),
            storage,
            policy_service,
        }
    }
}

impl FlowExecutor {
    pub fn new(storage: Arc<dyn ExecutorStorage>, policy_service: PolicyService) -> Self {
        Self {
            internal: Arc::new(FlowExecutorInternal::new(storage, policy_service)),
        }
    }

    pub fn invalidate_flow(&self, key: &FlowKey) {
        self.internal.executions.invalidate(key);
    }

    pub fn get_key(&self, session: &Session, flow: i32) -> FlowKey {
        FlowKey::new(session, flow)
    }

    pub async fn get_execution(
        &self,
        key: &FlowKey,
        start: bool,
    ) -> Result<Option<FlowExecution>, ApiError> {
        let execution = self.internal.executions.get(key);
        if execution.is_some() {
            return Ok(execution);
        }

        if start {
            self.start(key).await
        } else {
            Ok(None)
        }
    }

    pub async fn start(&self, key: &FlowKey) -> Result<Option<FlowExecution>, ApiError> {
        let flow = match self.internal.storage.get_flow(key.flow).await? {
            Some(flow) => flow,
            None => {
                return Ok(None);
            }
        };
        // let proxied = ::storage::create_proxied(&self.internal.storage_old);
        // if flow.entries.is_empty() {
        //     return None;
        // }
        // proxied.reverse_lookup(&*flow).await;
        // let storage = ::storage::create_freezed(proxied);
        let storage = self.internal.storage.clone();
        let context = ExecutionContext::new(key.session.clone(), storage.clone());
        let execution = FlowExecutionInternal {
            flow,
            context: RwLock::new(context),
            storage,
            current_entry_idx: parking_lot::Mutex::new(0),
            is_completed: AtomicBool::new(false),
            executor: self.clone(),
            key: key.clone(),
            policy_service: self.internal.policy_service.clone(),
        };
        let execution = FlowExecution(Arc::new(execution));
        self.internal
            .executions
            .insert(key.clone(), execution.clone());
        Ok(Some(execution))
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
                    ((context.start_time - date) > time::Duration::seconds(*max_age as i64)).into()
                }),
            PolicyKind::PasswordStrength => todo!(),
            PolicyKind::Expression(..) => todo!(),
        }
    }
}
