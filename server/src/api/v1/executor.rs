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
use uuid::Uuid;

use crate::{
    api::{sql_tx::Tx, ApiError, ApiErrorKind, AuthServiceData, ExecutorQuery},
    auth::Session,
    executor::{flow::FlowExecution, FlowExecutor},
    service::user::UserService,
    SharedState,
};
use model::{
    error::{FieldError, FieldType, SubmissionError},
    Flow, FlowData, PasswordBackend, PendingUser, Reference, StageKind, UserField,
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

async fn get_flow(
    session: Session,
    flow: Reference<Flow>,
    State(state): State<SharedState>,
    query: Option<ExecutorQuery>,
) -> Result<Json<FlowData>, ApiError> {
    let executor = state.executor();
    let key = executor
        .get_key(&session, flow)
        .ok_or(ApiErrorKind::MiscInternal.into_api())?;
    let execution = executor
        .get_execution(&key, true)
        .await
        .ok_or(ApiErrorKind::NotFound.into_api())?;
    let data = execution.data(None, query.unwrap_or_default()).await;
    Ok(Json(data))
}

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
    if let Err(err) = handle_submission(form, &mut tx, executor, &state.users(), &execution).await {
        match &err.kind {
            ApiErrorKind::SubmissionError(err) => Ok(Json(
                execution
                    .data(Some(err.clone()), query.unwrap_or_default())
                    .await,
            )
            .into_response()),
            _ => Err(err),
        }
    } else {
        execution.complete_current();
        complete(&mut tx, &execution, &state.auth_data(), &cookies, session).await?;
        tx.commit().await?;
        Ok(Redirect::to(uri.to_string().as_str()).into_response())
    }
}

async fn handle_submission(
    form: Value,
    tx: &mut Tx<Postgres>,
    _executor: &FlowExecutor,
    users: &UserService,
    execution: &FlowExecution,
) -> Result<(), ApiError> {
    let entry = execution.get_entry();
    let stage = execution.lookup_stage(&entry.stage).await;
    match &stage.kind {
        StageKind::Deny => return Ok(()),
        StageKind::Prompt { bindings: _ } => return Ok(()),
        StageKind::Identification {
            password: _,
            user_fields,
        } => {
            let uid = str_from_field(
                "uid",
                form.get("uid").ok_or(SubmissionError::MissingField {
                    field_name: "uid".into(),
                })?,
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
                        avatar_url: "".into(),
                        authenticated: false,
                    });
                }),
                None => {
                    return Err(SubmissionError::from(FieldError::Invalid {
                        field: "uid".into(),
                        message: "Failed to authenticate".to_owned(),
                    })
                    .into())
                }
            };
            return Ok(());
        }
        StageKind::UserLogin => return Ok(()),
        StageKind::UserLogout => return Ok(()),
        StageKind::UserWrite => return Ok(()),
        StageKind::Password { backends } => {
            let pending = match execution.get_context().pending.clone() {
                Some(v) => v,
                None => return Err(SubmissionError::NoPendingUser.into()),
            };
            let password = str_from_field(
                "password",
                form.get("password").ok_or(SubmissionError::MissingField {
                    field_name: "password".into(),
                })?,
            )?;
            if backends.contains(&PasswordBackend::Internal) {
                let res = query!("select password from users where uid = $1", pending.uid)
                    .fetch_optional(&mut *tx)
                    .await?;
                match res {
                    Some(res) => {
                        let hash = PasswordHash::parse(&res.password, Encoding::B64)?;
                        let is_valid =
                            match hash.verify_password(&[&argon2::Argon2::default()], password) {
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
            return Err(SubmissionError::Field(FieldError::Invalid {
                field: "password".into(),
                message: "Invalid Password".to_owned(),
            })
            .into());
        }
        StageKind::Consent { mode: _ } => return Ok(()),
    };
}

fn str_from_field<'a>(name: &'static str, value: &'a Value) -> Result<&'a String, SubmissionError> {
    match value {
        Value::String(value) => Ok(value),
        other => Err(SubmissionError::InvalidFieldType {
            field_name: name.into(),
            expected: FieldType::String,
            got: other.into(),
        }),
    }
}

async fn complete(
    tx: &mut Tx<Postgres>,
    execution: &FlowExecution,
    keys: &AuthServiceData,
    cookies: &Cookies,
    session: Session,
) -> Result<(), ApiError> {
    loop {
        if execution.is_completed() {
            break;
        }
        let entry = execution.get_entry();
        let stage = execution.lookup_stage(&entry.stage).await;
        if stage.kind.requires_input() {
            break;
        }
        match stage.kind {
            StageKind::UserLogin => {
                let (uid, is_authenticated): (Uuid, bool) = execution
                    .get_context()
                    .pending
                    .as_ref()
                    .map(|v| (v.uid.clone(), v.authenticated))
                    .ok_or(SubmissionError::NoPendingUser)?;
                if !is_authenticated {
                    continue;
                }
                let cookie = cookies
                    .get(SESSION_COOKIE_NAME)
                    .ok_or(ApiErrorKind::InvalidSessionCookie)?;
                let mut claims: Claims = decode_token(&keys.decoding_key, cookie.value())?.claims;
                claims.sub = Some(uid);
                claims.authenticated = true;
                set_session_cookie(&keys.encoding_key, cookies, &claims)?;
                query!(
                    "update sessions set user_id = $1 where uid = $2",
                    uid,
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
