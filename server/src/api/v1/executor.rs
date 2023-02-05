use async_trait::async_trait;
use poem::{
    get, handler,
    web::{Data, Json, Path},
    FromRequest, Request, RequestBody, Route,
};
use serde::Deserialize;

use crate::{
    api::{v1::ApiError, v1::ApiErrorKind},
    auth::Session,
    executor::{data::FlowData, FlowExecutor},
    model::{Flow, Reference},
};

pub fn setup_executor_router() -> Route {
    Route::new().at("/:flow_slug", get(get_flow))
}

#[derive(Deserialize)]
pub struct FlowParam {
    flow_slug: String,
}

#[async_trait]
impl<'a> FromRequest<'a> for Reference<Flow> {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> poem::Result<Self> {
        let path: Path<FlowParam> = Path::from_request(req, body).await?;
        let flow_slug = path.0.flow_slug;
        Ok(Reference::new_slug(flow_slug))
    }
}

#[handler]
async fn get_flow(
    session: Session,
    flow: Reference<Flow>,
    Data(executor): Data<&FlowExecutor>,
) -> Result<Json<FlowData>, ApiError> {
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let execution = executor
        .get_execution(&key)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let data = execution.data().await;
    Ok(Json(data))
}
