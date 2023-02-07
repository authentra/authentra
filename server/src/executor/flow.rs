use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use parking_lot::{lock_api::RwLockReadGuard, Mutex, RawRwLock, RwLock};

use crate::{
    api::ExecutorQuery,
    flow_storage::ReferenceLookup,
    model::{Flow, FlowEntry, Reference, Stage},
};

use super::{
    data::{FlowComponent, FlowData, FlowInfo},
    ExecutionContext, SubmissionError,
};

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
    pub async fn data(&self, error: Option<SubmissionError>, query: ExecutorQuery) -> FlowData {
        let flow = self.0.flow.as_ref();
        let entry = self.get_entry();
        let stage = self.lookup_stage(&entry.stage).await;
        let component = if self.0.is_completed.load(Ordering::Relaxed) {
            match query.next {
                Some(to) => FlowComponent::Redirect { to },
                None => FlowComponent::Error {
                    message: "Missing Redirect".to_owned(),
                },
            }
        } else {
            match stage.as_component(self).await {
                Some(v) => v,
                None => FlowComponent::AccessDenied {
                    message: "Internal error while constructing component".to_owned(),
                },
            }
        };
        FlowData {
            flow: FlowInfo {
                title: flow.title.clone(),
            },
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
}

pub(super) struct FlowExecutionInternal {
    pub(super) flow: Arc<Flow>,
    pub(super) context: RwLock<ExecutionContext>,
    pub(super) current_entry_idx: Mutex<usize>,
    pub(super) is_completed: AtomicBool,
}
