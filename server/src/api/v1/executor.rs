use std::sync::Arc;

use argon2::{password_hash::Encoding, PasswordHash};
use axum::{
    extract::{OriginalUri, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, MethodRouter},
    Form, Json, Router,
};

use serde_json::Value;
use sqlx::{query, Postgres};
use tower_cookies::Cookies;
use tracing::instrument;

use crate::{
    api::{sql_tx::Tx, ApiError, ApiErrorKind, AuthServiceData, ExecutorQuery},
    auth::Session,
    executor::{
        flow::{CheckContextRequest, FlowExecution},
        ExecutionError, FlowExecutor,
    },
    service::user::UserService,
    SharedState,
};
use model::{
    error::{FieldError, FieldErrorKind, FieldType, SubmissionError},
    Flow, FlowData, PasswordBackend, PendingUser, Reference, Stage, StageKind, UserField,
};

use super::{
    auth::{decode_token, set_session_cookie, Claims, SESSION_COOKIE_NAME},
    ping_handler,
};

pub fn setup_executor_router() -> Router<SharedState> {
    Router::new().route("/ping", get(ping_handler)).route(
        "/:flow_slug",
        MethodRouter::new().get(get_flow).post(post_flow),
    )
}

#[instrument(skip(state, session))]
async fn get_flow(
    session: Session,
    flow: Reference<Flow>,
    State(state): State<SharedState>,
    query: Option<ExecutorQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<FlowData>, ApiError> {
    let executor = state.executor();
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::MiscInternal.into_api())?;
    let execution = executor
        .get_execution(&key, true)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let connection = state.defaults().connection().await?;
    let context = CheckContextRequest {
        uri,
        query: query.unwrap_or_default(),
        user: session.get_user(&connection, &state).await?,
    };
    let data = execution.data(None, context).await;
    Ok(Json(data))
}

#[instrument(skip(state, session, form, cookies, uri, tx))]
async fn post_flow(
    session: Session,
    mut tx: Tx<Postgres>,
    flow: Reference<Flow>,
    State(state): State<SharedState>,
    OriginalUri(uri): OriginalUri,
    cookies: Cookies,
    query: Option<ExecutorQuery>,
    Form(form): Form<Value>,
) -> Result<Response, ApiError> {
    let executor = state.executor();
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let execution = executor
        .get_execution(&key, false)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let connection = state.defaults().connection().await?;
    let context = CheckContextRequest {
        uri: uri.clone(),
        query: query.unwrap_or_default(),
        user: session.get_user(&connection, &state).await?,
    };
    if let Err(err) = handle_submission(form, &mut tx, executor, &state.users(), &execution).await {
        match &err.kind {
            ApiErrorKind::SubmissionError(err) => {
                Ok(Json(execution.data(Some(err.clone()), context).await).into_response())
            }
            _ => Err(err),
        }
    } else {
        execution.complete_current();
        complete(&mut tx, &execution, &state.auth_data(), &cookies, session).await?;
        tx.commit().await?;
        Ok(Redirect::to(uri.to_string().as_str()).into_response())
    }
}

#[instrument(skip(form, tx, executor, users, execution))]
async fn handle_submission(
    form: Value,
    tx: &mut Tx<Postgres>,
    executor: &FlowExecutor,
    users: &UserService,
    execution: &FlowExecution,
) -> Result<(), ApiError> {
    let entry = execution.get_entry();
    let stage = execution.lookup_stage(&entry.stage).await;
    handle_stage(form, tx, executor, users, execution, stage).await
}

#[instrument(skip(form, tx, _executor, users, execution))]
async fn handle_stage(
    form: Value,
    tx: &mut Tx<Postgres>,
    _executor: &FlowExecutor,
    users: &UserService,
    execution: &FlowExecution,
    stage: Arc<Stage>,
) -> Result<(), ApiError> {
    match &stage.kind {
        StageKind::Deny => return Ok(()),
        StageKind::Prompt { bindings: _ } => return Ok(()),
        StageKind::Identification {
            password,
            user_fields,
        } => {
            let uid = str_from_field(
                "uid",
                form.get("uid")
                    .ok_or(SubmissionError::Field(FieldError::new(
                        "uid",
                        FieldErrorKind::Missing,
                    )))?,
            )?;
            let user = users
                .lookup_user(
                    &mut *tx,
                    uid,
                    user_fields.contains(&UserField::Name),
                    user_fields.contains(&UserField::Email),
                    user_fields.contains(&UserField::Uuid),
                )
                .await?;
            match user {
                Some(user) => execution.use_mut_context(move |ctx| {
                    ctx.pending = Some(PendingUser {
                        uid: user.uid,
                        name: user.name,
                        avatar_url: None,
                        authenticated: false,
                        is_admin: user.is_admin,
                    });
                }),
                None => {
                    return Err(SubmissionError::from(FieldError::new(
                        "uid",
                        FieldErrorKind::invalid("Failed to authenticate"),
                    ))
                    .into())
                }
            };
            if let Some(password) = password {
                let password = execution.lookup_stage(password).await;
                match &password.kind {
                    StageKind::Password { backends } => {
                        return handle_password_stage(&form, tx, execution, &backends).await;
                    }
                    _ => unreachable!("Is not password stage"),
                };
            }
            return Ok(());
        }
        StageKind::UserLogin => return Ok(()),
        StageKind::UserLogout => return Ok(()),
        StageKind::UserWrite => return Ok(()),
        StageKind::Password { backends } => {
            return handle_password_stage(&form, tx, execution, backends).await;
        }
        StageKind::Consent { mode: _ } => return Ok(()),
    };
}

#[instrument(skip(form, tx, execution))]
async fn handle_password_stage(
    form: &Value,
    tx: &mut Tx<Postgres>,
    execution: &FlowExecution,
    backends: &Vec<PasswordBackend>,
) -> Result<(), ApiError> {
    let pending = match execution.get_context().pending.clone() {
        Some(v) => v,
        None => return Err(SubmissionError::NoPendingUser.into()),
    };
    let password = str_from_field(
        "password",
        form.get("password")
            .ok_or(SubmissionError::Field(FieldError::new(
                "password",
                FieldErrorKind::Missing,
            )))?,
    )?;
    if backends.contains(&PasswordBackend::Internal) {
        let res = query!("select password from users where uid = $1", pending.uid)
            .fetch_optional(&mut *tx)
            .await?;
        match res {
            Some(res) => {
                let hash = PasswordHash::parse(&res.password, Encoding::B64)?;
                let is_valid = match hash.verify_password(&[&argon2::Argon2::default()], password) {
                    Ok(_) => true,
                    Err(err) => match err {
                        argon2::password_hash::Error::Password => false,
                        err => return Err(err.into()),
                    },
                };
                if is_valid {
                    execution.use_mut_context(|ctx| {
                        ctx.pending
                            .as_mut()
                            .map(|pending| pending.authenticated = true);
                    });
                    return Ok(());
                }
            }
            None => return Err(SubmissionError::NoPendingUser.into()),
        }
    }
    return Err(SubmissionError::Field(FieldError::new(
        "password",
        FieldErrorKind::invalid("Invalid Password"),
    ))
    .into());
}

fn str_from_field<'a>(name: &'static str, value: &'a Value) -> Result<&'a String, SubmissionError> {
    match value {
        Value::String(value) => Ok(value),
        other => Err(SubmissionError::Field(FieldError::new(
            name,
            FieldErrorKind::invalid_type(FieldType::String, other.into()),
        ))),
    }
}

#[instrument(skip(tx, execution, keys, cookies, session))]
async fn complete(
    tx: &mut Tx<Postgres>,
    execution: &FlowExecution,
    keys: &AuthServiceData,
    cookies: &Cookies,
    session: Session,
) -> Result<(), ApiError> {
    let mut iterations = 0;
    loop {
        if execution.is_completed() {
            break;
        }
        let entry = execution.get_entry();
        let stage = execution.lookup_stage(&entry.stage).await;
        if stage.kind.requires_input() {
            break;
        }
        if iterations > 40 {
            tracing::error!(
                stage = ?stage,
                "Detected long running loop while completing stage",
            );
            break;
        }
        iterations += 1;
        match stage.kind {
            StageKind::UserLogin => {
                let user = execution
                    .get_context()
                    .pending
                    .as_ref()
                    .cloned()
                    .ok_or(SubmissionError::NoPendingUser)?;
                if !user.authenticated {
                    execution.set_error(ExecutionError {
                        stage: Some(entry.stage.clone()),
                        message: "Authentication of pending user failed".into(),
                    });
                    break;
                }
                let cookie = cookies
                    .get(SESSION_COOKIE_NAME)
                    .ok_or(ApiErrorKind::InvalidSessionCookie)?;
                let mut claims: Claims = decode_token(&keys.decoding_key, cookie.value())?.claims;
                claims.sub = Some(user.uid);
                claims.authenticated = true;
                claims.is_admin = user.is_admin;
                set_session_cookie(&keys.encoding_key, cookies, &claims)?;
                query!(
                    "update sessions set user_id = $1 where uid = $2",
                    user.uid,
                    session.session_id
                )
                .execute(&mut *tx)
                .await?;
            }
            StageKind::UserLogout => todo!(),
            StageKind::UserWrite => todo!(),
            _ => unreachable!("Encountered client side stage"),
        }
        execution.complete_current();
    }
    Ok(())
}
