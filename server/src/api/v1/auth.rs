use std::fmt::Debug;

use async_trait::async_trait;
use http::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey, TokenData, Validation};
use once_cell::sync::Lazy;
use poem::{
    get, handler, post, Endpoint, FromRequest, IntoResponse, Middleware, Request, RequestBody,
    Response, Route,
};
use serde::{Deserialize, Serialize};
use sqlx::{query, Postgres};
use uuid::Uuid;

use crate::{api::sql_tx::Tx, auth::Session};

mod login;
mod logout;
mod register;

const SESSION_COOKIE_NAME: &str = "session";

pub(super) fn setup_auth_router() -> Route {
    Route::new()
        .at("/login", post(login::login))
        .at("/logout", post(logout::logout))
        .at("/register", register::register)
        .at("/", get(test))
}

#[handler]
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

impl<E: Endpoint> Middleware<E> for AuthLayer {
    type Output = AuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        AuthEndpoint {
            data: self.data.clone(),
            inner: ep,
        }
    }
}

pub enum AuthExtension {
    Valid(AuthExtensionData),
    MissingCookie,
    InvalidCookieProperties,
    InvalidCookieFormat,
}

#[derive(Debug, Clone)]
pub struct AuthExtensionData {
    pub header: jsonwebtoken::Header,
    pub claims: Claims,
}

#[derive(Clone)]
pub struct AuthServiceData {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
}

impl Debug for AuthServiceData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthServiceData").finish()
    }
}

pub struct AuthEndpoint<E: Endpoint> {
    data: AuthServiceData,
    inner: E,
}

#[async_trait]
impl<E: Endpoint> Endpoint for AuthEndpoint<E> {
    type Output = Response;
    async fn call(&self, mut req: Request) -> poem::Result<Self::Output> {
        if let Err(err) = handle_auth(&mut req, &self.data) {
            return Ok(err);
        }
        self.inner.call(req).await.map(IntoResponse::into_response)
    }
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

fn handle_auth(req: &mut poem::Request, data: &AuthServiceData) -> Result<(), Response> {
    let cookies = req.cookie();
    let Some(cookie) = cookies.get(SESSION_COOKIE_NAME) else {
        req.extensions_mut().insert(AuthExtension::MissingCookie);
        return Ok(()) };
    let value = cookie.value_str();
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

fn get_auth_extension_data(req: &Request) -> Result<AuthExtensionData, Response> {
    let Some(extension): Option<&AuthExtension> = req.extensions().get() else {
            tracing::error!("Missing AuthMiddleware");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        };
    let data = match extension {
        AuthExtension::Valid(data) => data,
        AuthExtension::MissingCookie => {
            return Err((StatusCode::UNAUTHORIZED, "Missing session cookie").into_response())
        }
        AuthExtension::InvalidCookieProperties => {
            return Err(
                (StatusCode::BAD_REQUEST, "Session cookie invalid parameters").into_response(),
            );
        }
        AuthExtension::InvalidCookieFormat => {
            return Err((StatusCode::BAD_REQUEST, "Session cookie format").into_response());
        }
    };
    Ok(data.to_owned())
}

#[async_trait]
impl<'a> FromRequest<'a> for Session {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        let data = get_auth_extension_data(req).map_err(|err| poem::Error::from_response(err))?;
        let claims = data.claims;
        let tx: Result<Tx<Postgres>, _> = Tx::from_request_without_body(req).await;
        if let Err(err) = tx {
            return Err(err);
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
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        if let Some(res) = res {
            Ok(Session {
                session_id: claims.sid,
                user_id: res.user_id,
            })
        } else {
            Err(StatusCode::UNAUTHORIZED.into())
        }
    }
}
