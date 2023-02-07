use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use derive_more::Display;
use moka::sync::Cache;
use parking_lot::RwLock;
use serde::Serialize;

use crate::{
    auth::Session,
    flow_storage::{FlowStorage, FreezedStorage, ReferenceLookup, ReverseLookup},
    model::{Flow, Reference, ReferenceKind},
};

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
#[derive(Debug, Clone, Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldError {
    Invalid {
        field: &'static str,
        message: String,
    },
}

#[derive(Clone)]
pub struct FlowExecutor {
    internal: Arc<FlowExecutorInternal>,
}

struct FlowExecutorInternal {
    executions: Cache<FlowKey, FlowExecution>,
    storage: FlowStorage,
}

impl FlowExecutorInternal {
    pub fn new(storage: FlowStorage) -> Self {
        Self {
            executions: Cache::builder()
                .time_to_idle(TIME_TO_IDLE.clone())
                .time_to_live(TIME_TO_LIVE.clone())
                .build(),
            storage,
        }
    }
}

impl FlowExecutor {
    pub fn new(storage: FlowStorage) -> Self {
        Self {
            internal: Arc::new(FlowExecutorInternal::new(storage)),
        }
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
            current_entry_idx: AtomicUsize::new(0),
        };
        let execution = FlowExecution(Arc::new(execution));
        self.internal
            .executions
            .insert(key.clone(), execution.clone());
        Some(execution)
    }

    // pub fn init(&mut self) {}
}
