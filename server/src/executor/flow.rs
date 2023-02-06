use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use http::StatusCode;
use poem::{IntoResponse, Request, Response};

use crate::{
    api::{ApiError, Redirect},
    flow_storage::ReferenceLookup,
    model::{Flow, FlowEntry, Reference, Stage},
};

use super::{
    data::{FlowComponent, FlowData, FlowInfo},
    ExecutionContext, FlowExecutor,
};

#[derive(Clone)]
pub struct FlowExecution(pub(super) Arc<FlowExecutionInternal>);

impl FlowExecution {
    pub async fn data(&self) -> FlowData {
        let flow = self.0.flow.as_ref();
        let entry = self.get_entry();
        let stage = self.lookup_stage(&entry.stage).await;
        FlowData {
            flow: FlowInfo {
                title: flow.title.clone(),
            },
            component: FlowComponent::AccessDenied {
                message: "Not implemented".to_owned(),
            },
            pending_user: None,
        }
    }
    pub fn get_entry(&self) -> &FlowEntry {
        let entry_idx = self.0.current_entry_idx.load(Ordering::Acquire);
        let Some(entry) =self.0.flow.entries.get(entry_idx) else { panic!("Entry index out of bounds") };
        entry
    }

    async fn lookup_stage(&self, reference: &Reference<Stage>) -> Arc<Stage> {
        match self.0.context.storage.lookup_reference(reference).await {
            Some(v) => v,
            None => panic!("Missing stage in storage"),
        }
    }

    pub async fn next(&self, request: &Request) -> poem::Result<Response, ApiError> {
        Ok(Redirect::found(request.uri().path()).into_response())
    }
}

pub(super) struct FlowExecutionInternal {
    pub(super) flow: Arc<Flow>,
    pub(super) context: ExecutionContext,
    pub(super) current_entry_idx: AtomicUsize,
    pub(super) executor: FlowExecutor,
}
