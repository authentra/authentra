use axum::{
    extract::{BodyStream, Path, Query, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use deadpool_postgres::GenericClient;
use futures::StreamExt;
use http::StatusCode;
use model::{PartialPolicy, PolicyKind, PolicyKindSimple};
use policy_engine::{compile, encode_base64};
use serde::Deserialize;
use tracing::instrument;

use crate::{api::ApiError, service::policy::DUMMY_SCOPE, SharedState};

use super::auth::AdminSession;

pub fn setup_policy_router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list))
        .route("/:slug/expiration", post(create_expiration))
        .route("/:slug/expression", post(create_expression))
}

#[instrument(skip(state))]
pub async fn list(
    _: AdminSession,
    State(state): State<SharedState>,
) -> Result<Json<Vec<PartialPolicy>>, ApiError> {
    let service = state.storage();
    let policies = service
        .list_policies()
        .await?
        .into_iter()
        .map(PartialPolicy::from);
    Ok(Json(policies.collect()))
}

#[derive(Deserialize)]
struct ExpirationQuery {
    max_age: i32,
}

async fn create_expiration(
    _: AdminSession,
    Path(slug): Path<String>,
    Query(query): Query<ExpirationQuery>,
    State(state): State<SharedState>,
) -> Result<Response, ApiError> {
    if query.max_age < 0 {
        return Ok((StatusCode::BAD_REQUEST, "max_age must be positive").into_response());
    }
    let mut connection = state.defaults().connection().await?;
    let connection = connection.transaction().await?;
    let partial = create(
        slug,
        PolicyKind::PasswordExpiry {
            max_age: query.max_age,
        },
        &connection,
    )
    .await?;
    connection.commit().await?;
    Ok(Json(partial).into_response())
}

const MAX_EXPRESSION_LEN: usize = 2048;

async fn create_expression(
    _: AdminSession,
    Path(slug): Path<String>,
    State(state): State<SharedState>,
    mut stream: BodyStream,
) -> Result<Response, ApiError> {
    let mut connection = state.defaults().connection().await?;
    let connection = connection.transaction().await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > MAX_EXPRESSION_LEN {
            return Ok((StatusCode::PAYLOAD_TOO_LARGE, "Request body too large").into_response());
        }
        bytes.extend(chunk);
    }
    let expression = match String::from_utf8(bytes) {
        Ok(v) => v,
        Err(_) => return Ok((StatusCode::BAD_REQUEST, "Requset body must be utf8").into_response()),
    };
    let expression = encode_base64(&expression);
    match compile(&expression, &DUMMY_SCOPE) {
        Ok(_) => {}
        Err(err) => match err {
            policy_engine::ExpressionCompilationError::InvalidBase64 => {
                return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
            policy_engine::ExpressionCompilationError::Parse(parse) => {
                return Ok((StatusCode::BAD_REQUEST, format!("{parse}")).into_response());
            }
        },
    };
    let partial = create(slug, PolicyKind::Expression(expression), &connection).await?;
    connection.commit().await?;
    Ok(Json(partial).into_response())
}

async fn create<C: GenericClient>(
    slug: String,
    kind: PolicyKind,
    client: &C,
) -> Result<PartialPolicy, ApiError> {
    let simple_kind = PolicyKindSimple::from(&kind);
    // let connection = connection.transaction().await?;
    let statement = client
        .prepare_cached("insert into policies(slug, kind) values($1, $2) returning uid")
        .await?;
    let uid: i32 = client
        .query_one(&statement, &[&slug, &simple_kind])
        .await?
        .get(0);
    match kind {
        PolicyKind::PasswordExpiry { max_age } => {
            let statement = client
                .prepare_cached(
                    "insert into password_expiration_policies(max_age) values ($1) returning uid",
                )
                .await?;
            let sub_uid: i32 = client.query_one(&statement, &[&max_age]).await?.get(0);

            let statement = client
                .prepare_cached("update policies set password_expiration=$1 where uid = $2")
                .await?;
            client.execute(&statement, &[&sub_uid, &uid]).await?;
        }
        PolicyKind::PasswordStrength => todo!(),
        PolicyKind::Expression(expression) => {
            let statement = client
                .prepare_cached(
                    "insert into expression_policies(expression) values ($1) returning uid",
                )
                .await?;
            let sub_uid: i32 = client.query_one(&statement, &[&expression]).await?.get(0);

            let statement = client
                .prepare_cached("update policies set expression=$1 where uid = $2")
                .await?;
            client.execute(&statement, &[&sub_uid, &uid]).await?;
        }
    }
    Ok(PartialPolicy {
        uid,
        slug,
        kind: simple_kind,
    })
}
