use async_trait::async_trait;
use axum::{
    extract::{rejection::PathRejection, FromRequestParts, OriginalUri, Path},
    response::{IntoResponse, Redirect, Response},
    routing::{get, MethodRouter},
    Extension, Form, Json, Router,
};
use derive_more::{Display, Error};
use http::{request::Parts, Uri};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Postgres;

use crate::{
    api::{sql_tx::Tx, ApiError, ApiErrorKind},
    auth::Session,
    executor::{
        data::{FlowData, PendingUser},
        flow::FlowExecution,
        FieldError, FlowExecutor,
    },
    model::{Flow, Reference, UserField},
    service::user::UserService,
};

use super::ping_handler;

pub fn setup_executor_router() -> Router {
    Router::new().route("/ping", get(ping_handler)).route(
        "/:flow_slug",
        MethodRouter::new().get(get_flow).post(post_flow),
    )
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
    let data = execution.data(None).await;
    Ok(Json(data))
}

async fn post_flow(
    session: Session,
    tx: Tx<Postgres>,
    flow: Reference<Flow>,
    Extension(executor): Extension<FlowExecutor>,
    Extension(users): Extension<UserService>,
    OriginalUri(uri): OriginalUri,
    Form(form): Form<Value>,
) -> Result<Response, ApiError> {
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let execution = executor
        .get_execution(&key, false)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let error = handle_submission(form, tx, executor, users, &execution).await?;
    Ok(match error {
        Some(err) => Json(execution.data(Some(err)).await).into_response(),
        None => Redirect::to(uri.path()).into_response(),
    })
}

async fn handle_submission(
    form: Value,
    mut tx: Tx<Postgres>,
    _executor: FlowExecutor,
    users: UserService,
    execution: &FlowExecution,
) -> Result<Option<FieldError>, ApiError> {
    let entry = execution.get_entry();
    let stage = execution.lookup_stage(&entry.stage).await;
    match &stage.kind {
        crate::model::StageKind::Deny => return Ok(None),
        crate::model::StageKind::Prompt { bindings: _ } => return Ok(None),
        crate::model::StageKind::Identification {
            password: _,
            user_fields,
        } => {
            let uid = str_from_field(
                "uid",
                form.get("uid")
                    .ok_or(SubmissionValidationError::MissingField { field_name: "uid" })?,
            )?;
            let user = users
                .lookup_user(
                    &mut tx,
                    uid,
                    user_fields.contains(&UserField::Name),
                    user_fields.contains(&UserField::Email),
                    user_fields.contains(&UserField::Uuid),
                )
                .await?;
            match user {
                Some(user) => execution.use_mut_context(move |ctx| {
                    ctx.pending = Some(PendingUser {
                        name: user.name,
                        avatar_url: "".into(),
                    });
                }),
                None => {
                    return Ok(Some(FieldError::Invalid {
                        field: "uid",
                        message: "Failed to authenticate".to_owned(),
                    }))
                }
            };
        }
        crate::model::StageKind::UserLogin => return Ok(None),
        crate::model::StageKind::UserLogout => return Ok(None),
        crate::model::StageKind::UserWrite => return Ok(None),
        crate::model::StageKind::Password { backends: _ } => return Ok(None),
        crate::model::StageKind::Consent { mode: _ } => return Ok(None),
    };
    Ok(None)
}

fn str_from_field<'a>(
    name: &'static str,
    value: &'a Value,
) -> Result<&'a String, SubmissionValidationError> {
    match value {
        Value::String(value) => Ok(value),
        other => Err(SubmissionValidationError::InvalidFieldType {
            field_name: name,
            expected: FieldType::String,
            got: other.into(),
        }),
    }
}

#[derive(Debug, Clone, Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Null,
    Boolean,
    String,
    Number,
    Object,
    Array,
}

impl<'a> From<&'a Value> for FieldType {
    fn from(value: &'a Value) -> Self {
        match value {
            Value::Null => FieldType::Null,
            Value::Bool(_) => FieldType::Boolean,
            Value::Number(_) => FieldType::Number,
            Value::String(_) => FieldType::String,
            Value::Array(_) => FieldType::Array,
            Value::Object(_) => FieldType::Object,
        }
    }
}

#[derive(Debug, Clone, Error, Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionValidationError {
    MissingField {
        field_name: &'static str,
    },
    InvalidFieldType {
        field_name: &'static str,
        expected: FieldType,
        got: FieldType,
    },
}
