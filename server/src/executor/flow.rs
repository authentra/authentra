use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use parking_lot::{lock_api::RwLockReadGuard, RawRwLock, RwLock};

use crate::{
    flow_storage::ReferenceLookup,
    model::{Flow, FlowEntry, Reference, Stage},
};

use super::{
    data::{FlowComponent, FlowData, FlowInfo},
    ExecutionContext, FieldError,
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
    pub async fn data(&self, error: Option<FieldError>) -> FlowData {
        let flow = self.0.flow.as_ref();
        let entry = self.get_entry();
        let stage = self.lookup_stage(&entry.stage).await;
        let component = match stage.as_component(self).await {
            Some(v) => v,
            None => FlowComponent::AccessDenied {
                message: "Internal error while constructing component".to_owned(),
            },
        };
        FlowData {
            flow: FlowInfo {
                title: flow.title.clone(),
            },
            component,
            pending_user: self.0.context.read().pending.clone(),
            field_error: error,
        }
    }
    pub fn get_entry(&self) -> &FlowEntry {
        let entry_idx = self.0.current_entry_idx.load(Ordering::Acquire);
        let Some(entry) =self.0.flow.entries.get(entry_idx) else { panic!("Entry index out of bounds") };
        entry
    }

    pub async fn lookup_stage(&self, reference: &Reference<Stage>) -> Arc<Stage> {
        match self
            .0
            .context
            .read()
            .storage
            .lookup_reference(reference)
            .await
        {
            Some(v) => v,
            None => panic!("Missing stage in storage"),
        }
    }
}

pub(super) struct FlowExecutionInternal {
    pub(super) flow: Arc<Flow>,
    pub(super) context: RwLock<ExecutionContext>,
    pub(super) current_entry_idx: AtomicUsize,
}
