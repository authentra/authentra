use std::fmt::Debug;

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use futures::future::BoxFuture;

use http::{request::Parts, Request, StatusCode};
use jsonwebtoken::Validation;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::{query, Postgres};
use tower::{Layer, Service};
use uuid::Uuid;

use crate::{
    api::{sql_tx::Tx, AuthServiceData},
    auth::Session,
};

mod login;
mod logout;
mod register;

const SESSION_COOKIE_NAME: &str = "session";

pub(super) fn setup_auth_router() -> Router {
    Router::new()
        .route("/login", post(login::login))
        .route("/logout", post(logout::logout))
        .route("/register", post(register::register))
        .route("/", get(test))
}

pub async fn test(_session: Session) -> Response {
    StatusCode::OK.into_response()
}

#[derive(Debug, Clone)]
pub struct AuthLayer {
    data: AuthServiceData,
}

impl AuthLayer {
    pub fn new(data: AuthServiceData) -> Self {
        Self { data }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            data: self.data.clone(),
            inner,
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    data: AuthServiceData,
    inner: S,
}
impl<S, ReqBody: Send + 'static> Service<Request<ReqBody>> for AuthService<S>
where
    S: Service<Request<ReqBody>> + Send + Clone + 'static,
    S::Response: IntoResponse,
    S::Future: Send + 'static,
{
    type Response = Response;

    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let data = self.data.clone();
        Box::pin(async move {
            if let Err(err) = handle_auth(&mut req, &data) {
                return Ok(err);
            }
            Ok(inner.call(req).await.map(IntoResponse::into_response)?)
        })
    }
}

pub enum AuthExtension {
    Valid(AuthExtensionData),
    MissingCookie,
    InvalidCookieFormat,
}

#[derive(Debug, Clone)]
pub struct AuthExtensionData {
    pub header: jsonwebtoken::Header,
    pub claims: Claims,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Oauth2Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
    iat: u64,
    auth_time: u64,
    acr: String,

    email: String,
    email_verified: bool,

    name: String,
    given_name: String,
    family_name: String,
    middle_name: String,
    nickname: String,
    preferred_username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    sid: String,
    iss: String,
    sub: Option<Uuid>,
    authenticated: bool,
}

fn response(code: StatusCode, body: &'static str) -> Response {
    (code, body).into_response()
}

const VALIDATION: Lazy<Validation> = Lazy::new(|| {
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_required_spec_claims(&["iss"]);
    validation.set_issuer(&["authust"]);
    validation
});

fn handle_auth<B>(req: &mut Request<B>, data: &AuthServiceData) -> Result<(), Response> {
    let cookies = CookieJar::from_headers(req.headers());
    let Some(cookie) = cookies.get(SESSION_COOKIE_NAME) else {
        req.extensions_mut().insert(AuthExtension::MissingCookie);
        return Ok(()) };
    let value = cookie.value();
    let token = match jsonwebtoken::decode(value, &data.decoding_key, &VALIDATION) {
        Ok(token) => token,
        Err(err) => {
            req.extensions_mut()
                .insert(AuthExtension::InvalidCookieFormat);
            tracing::error!("JWT Error: {err}");
            return Err(response(
                StatusCode::FORBIDDEN,
                "Session cookie validation error",
            ));
        }
    };
    req.extensions_mut()
        .insert(AuthExtension::Valid(AuthExtensionData {
            header: token.header,
            claims: token.claims,
        }));
    Ok(())
}

fn get_auth_extension_data(req: &mut Parts) -> Result<AuthExtensionData, Response> {
    let Some(extension): Option<&AuthExtension> = req.extensions.get() else {
            tracing::error!("Missing AuthMiddleware");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        };
    let data = match extension {
        AuthExtension::Valid(data) => data,
        AuthExtension::MissingCookie => {
            return Err((StatusCode::UNAUTHORIZED, "Missing session cookie").into_response())
        }
        AuthExtension::InvalidCookieFormat => {
            return Err((StatusCode::BAD_REQUEST, "Session cookie format").into_response());
        }
    };
    Ok(data.to_owned())
}

#[async_trait]
impl FromRequestParts<()> for Session {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let data = get_auth_extension_data(parts)?;
        let claims = data.claims;
        let tx: Result<Tx<Postgres>, _> = Tx::from_request_parts(parts, state).await;
        if let Err(err) = tx {
            return Err(err.into_response());
        }
        let mut tx = tx.expect("Rust bug");
        let res = query!(
            "select user_id as \"user_id?: Uuid\" from sessions where uid = $1",
            claims.sid
        )
        .fetch_optional(&mut tx)
        .await
        .map_err(|err| {
            tracing::error!("{err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;
        if let Some(res) = res {
            Ok(Session {
                session_id: claims.sid,
                user_id: res.user_id,
            })
        } else {
            Err(StatusCode::UNAUTHORIZED.into_response())
        }
    }
}
