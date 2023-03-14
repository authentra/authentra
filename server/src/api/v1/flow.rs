use axum::{extract::State, routing::get, Json, Router};
use model::{AuthenticationRequirement, Flow, FlowDesignation, FlowQuery};
use serde::Deserialize;
use storage::datacache::{Data, DataStorage, LookupRef};

use crate::{
    api::{errors::NOT_FOUND, ApiError, ApiErrorKind, RefWrapper},
    SharedState,
};

pub fn setup_flow_router() -> Router<SharedState> {
    Router::new().route("/:flow_slug", get(get_flow))
}

async fn get_flow(
    RefWrapper(flow): RefWrapper<Flow>,
    State(state): State<SharedState>,
) -> Result<Json<Data<Flow>>, ApiError> {
    state
        .storage()
        .lookup(&flow)
        .await
        .map(Json)
        .ok_or(NOT_FOUND)
}
#[derive(Debug, Deserialize)]
struct UpdateModel {
    title: Option<String>,
    designation: Option<FlowDesignation>,
    authentication: Option<AuthenticationRequirement>,
}

async fn modify_flow(
    RefWrapper(flow): RefWrapper<Flow>,
    State(state): State<SharedState>,
    Json(body): Json<UpdateModel>,
) -> Result<(), ApiError> {
    let storage = state.storage();
    let flow = storage.lookup(&flow).await.map(Json).ok_or(NOT_FOUND)?;
    let mut has_changed = false;
    if let Some(title) = body.title {
        if title != flow.title {}
    }

    if has_changed {
        let flow_storage = storage
            .get_for_data::<Flow>()
            .ok_or(ApiErrorKind::MiscInternal("missing flow storage"))?;
        let query = FlowQuery::uid(flow.uid);
        flow_storage.invalidate(&query).await?;
        flow_storage.find_one(&query).await?;
    }

    Ok(())
}
