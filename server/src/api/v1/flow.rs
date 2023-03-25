use axum::{
    extract::{Path, State},
    routing::MethodRouter,
    Json, Router,
};
use model::{AuthenticationRequirement, Flow, FlowDesignation, FlowQuery};
use serde::Deserialize;
use storage::datacache::{Data, DataStorage, LookupRef};

use crate::{
    api::{errors::NOT_FOUND, ApiError, ApiErrorKind, RefWrapper},
    SharedState,
};

use super::auth::AdminSession;

pub fn setup_flow_router() -> Router<SharedState> {
    Router::new()
        .route(
            "/:flow_uid",
            MethodRouter::new()
                .get(get_flow)
                .put(update_flow)
                .delete(delete_flow),
        )
        .route("/", MethodRouter::new().get(get_flows).post(create_flow))
}

async fn get_flows(
    _: AdminSession,
    State(state): State<SharedState>,
) -> Result<Json<Vec<Data<Flow>>>, ApiError> {
    state
        .storage_old()
        .get_for_data::<Flow>()
        .ok_or(ApiErrorKind::MiscInternal("Missing Flowstorage").into_api())?
        .find_all(None)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn get_flow(
    _: AdminSession,
    RefWrapper(flow): RefWrapper<Flow>,
    State(state): State<SharedState>,
) -> Result<Json<Data<Flow>>, ApiError> {
    state
        .storage_old()
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

async fn update_flow(
    _: AdminSession,
    RefWrapper(flow): RefWrapper<Flow>,
    State(state): State<SharedState>,
    Json(body): Json<UpdateModel>,
) -> Result<(), ApiError> {
    let storage = state.storage_old();
    let flow = storage.lookup(&flow).await.map(Json).ok_or(NOT_FOUND)?;
    let mut has_changed = false;
    let mut connection = state.defaults().connection().await?;
    let tx = connection.transaction().await?;
    let uid = flow.uid;
    if let Some(title) = body.title {
        if title != flow.title {
            has_changed = true;
            let statement = tx
                .prepare_cached("update flows set title = $1 where uid = $2")
                .await?;
            tx.execute(&statement, &[&title, &uid]).await?;
        }
    }
    if let Some(designation) = body.designation {
        if designation != flow.designation {
            has_changed = true;
            let statement = tx
                .prepare_cached("update flows set designation = $1 where uid = $2")
                .await?;
            tx.execute(&statement, &[&designation, &uid]).await?;
        }
    }
    if let Some(authentication) = body.authentication {
        if authentication != flow.authentication {
            has_changed = true;
            let statement = tx
                .prepare_cached("update flows set authentication = $1 where uid = $2")
                .await?;
            tx.execute(&statement, &[&authentication, &uid]).await?;
        }
    }

    if has_changed {
        let flow_storage = storage
            .get_for_data::<Flow>()
            .ok_or(ApiErrorKind::MiscInternal("missing flow storage"))?;
        let query = FlowQuery::uid(flow.uid);
        tx.commit().await?;
        flow_storage.invalidate(&query).await?;
        flow_storage.find_one(&query).await?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct CreateModel {
    slug: String,
    title: String,
    designation: FlowDesignation,
    authentication: AuthenticationRequirement,
}

async fn create_flow(
    _: AdminSession,
    State(state): State<SharedState>,
    Json(body): Json<CreateModel>,
) -> Result<Json<Data<Flow>>, ApiError> {
    let mut connection = state.defaults().connection().await?;
    let tx = connection.transaction().await?;
    let statement = tx.prepare_cached("insert into flows(slug,title,designation, authentication) values($1,$2,$3,$4) returning uid").await?;
    let row = tx
        .query_one(
            &statement,
            &[
                &body.slug,
                &body.title,
                &body.designation,
                &body.authentication,
            ],
        )
        .await?;
    let uid: i32 = row.get(0);
    tx.commit().await?;
    let flow = state
        .storage_old()
        .get_for_data::<Flow>()
        .ok_or(ApiErrorKind::MiscInternal("Missing Flowstorage").into_api())?
        .find_one(&FlowQuery::uid(uid))
        .await?;
    Ok(Json(flow))
}

async fn delete_flow(
    _: AdminSession,
    Path(uid): Path<i32>,
    State(state): State<SharedState>,
) -> Result<(), ApiError> {
    let mut connection = state.defaults().connection().await?;
    let tx = connection.transaction().await?;
    let statement = tx
        .prepare_cached("delete from flows where uid = $1")
        .await?;
    let affected = tx.execute(&statement, &[&uid]).await?;
    if affected == 1 {
        tx.commit().await?;
        Ok(())
    } else if affected == 0 {
        Err(NOT_FOUND)
    } else {
        Err(ApiErrorKind::MiscInternal("More than one row affected on flow deletion").into_api())
    }
}
