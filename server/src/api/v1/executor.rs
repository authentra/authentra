use async_trait::async_trait;
use axum::{
    extract::{rejection::PathRejection, FromRequestParts, Path},
    response::Redirect,
    routing::get,
    Extension, Form, Json, Router,
};
use http::{request::Parts, Uri};
use serde::Deserialize;

use crate::{
    api::{ApiError, ApiErrorKind},
    auth::Session,
    executor::{data::FlowData, FlowExecutor},
    model::{Flow, Reference},
};

use super::ping_handler;

pub fn setup_executor_router() -> Router {
    Router::new()
        .route("/ping", get(ping_handler))
        .route("/:flow_slug", get(get_flow))
}

#[derive(Deserialize)]
pub struct FlowParam {
    flow_slug: String,
}

#[async_trait]
impl FromRequestParts<()> for Reference<Flow> {
    type Rejection = PathRejection;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let path: Path<FlowParam> = Path::from_request_parts(parts, state).await?;
        let flow_slug = path.0.flow_slug;
        Ok(Reference::new_slug(flow_slug))
    }
}

async fn get_flow(
    session: Session,
    flow: Reference<Flow>,
    Extension(executor): Extension<FlowExecutor>,
) -> Result<Json<FlowData>, ApiError> {
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::MiscInternal.into_api())?;
    let execution = executor
        .get_execution(&key, true)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let data = execution.data().await;
    Ok(Json(data))
}
async fn post_flow(
    session: Session,
    flow: Reference<Flow>,
    Extension(executor): Extension<FlowExecutor>,
    uri: &Uri,
    Form(form): Form<serde_json::Value>,
) -> Result<Redirect, ApiError> {
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let _execution = executor
        .get_execution(&key, false)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    tracing::info!("{form:#?}");
    Ok(Redirect::to(uri.path()))
}
