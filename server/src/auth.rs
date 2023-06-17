use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::CookieJar;
use derive_more::Display;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::Lazy;
use postgres_types::{FromSql, ToSql};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{instrument, Span};
use uuid::Uuid;

use crate::{
    error::{Error, ErrorKind},
    AppResult, AppState,
};

pub const SESSION_COOKIE: &str = "session_token";
pub const REFRESH_COOKIE: &str = "refresh_token";

pub const ISSUER: &str = "authust";
static EXPIRATION_DURATION: Duration = Duration::from_secs(2 * 60);

static JWT_ALGO: Algorithm = Algorithm::HS256;

static VALIDATION: Lazy<Validation> = Lazy::new(|| {
    let mut validation = Validation::new(JWT_ALGO);
    validation.set_required_spec_claims(&["exp", "nbf", "iss", "sub"]);
    validation.set_issuer(&[ISSUER]);
    validation
});

pub fn jwt_header() -> Header {
    Header::new(JWT_ALGO)
}

static BEARER_AUTH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("^Bearer ([a-zA-Z0-9-_=.]{16,})$").unwrap());

pub struct AuthState {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl AuthState {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
    pub fn encoding(&self) -> &EncodingKey {
        &self.encoding
    }
    pub fn decoding(&self) -> &DecodingKey {
        &self.decoding
    }
}
#[derive(Debug, Display, Deserialize, Serialize, ToSql, FromSql, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[postgres(name = "user_roles")]
pub enum UserRole {
    #[postgres(name = "logs")]
    #[display("logs")]
    Logs,
    #[postgres(name = "developer")]
    #[display("developer")]
    Developer,
    #[postgres(name = "admin")]
    #[display("admin")]
    Admin,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub exp: u64,
    pub nbf: u64,
    pub sub: Uuid,
    pub sid: Uuid,
    pub aal: u8,
    pub authust: AuthustClaims,
}

#[derive(Serialize, Deserialize)]
pub struct AuthustClaims {
    pub roles: Vec<UserRole>,
}

impl Claims {
    pub fn new(user: Uuid, session: Uuid, authust: AuthustClaims) -> Self {
        Self {
            iss: ISSUER.into(),
            exp: (SystemTime::now() + EXPIRATION_DURATION)
                .duration_since(UNIX_EPOCH)
                .expect("Failed to get time since epoch")
                .as_secs(),
            nbf: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Failed to get time since epoch")
                .as_secs(),
            sub: user,
            sid: session,
            aal: 0,
            authust,
        }
    }
}

#[derive(Debug, Display)]
pub enum AuthError {
    #[display("Authentication cookie missing")]
    MissingCookie,
    #[display("Authorization header missing")]
    MissingHeader,
    #[display("Authorization header is invalid")]
    InvalidHeader,
    #[display("Invalid session")]
    InvalidSession,
    #[display("Invalid credentials")]
    InvalidCredentials,
    #[display("Claims missing in session info")]
    ClaimsMissingInInfo,
}

pub struct SessionInfo {
    pub id: Uuid,
    pub user: Uuid,
    pub claims: Option<Claims>,
}

impl SessionInfo {
    pub fn check_admin(&self) -> AppResult<()> {
        self.check_role(UserRole::Admin)
    }
    pub fn check_developer(&self) -> AppResult<()> {
        self.check_role(UserRole::Developer)
    }
    #[inline(always)]
    pub fn has_role(&self, role: UserRole) -> bool {
        if let Some(claims) = &self.claims {
            if claims.authust.roles.contains(&role)
                || claims.authust.roles.contains(&UserRole::Admin)
            {
                return true;
            }
        }
        false
    }
    #[inline(always)]
    #[instrument(skip_all, fields(roles,required = %role))]
    pub fn check_role(&self, role: UserRole) -> AppResult<()> {
        let current = Span::current();
        if let Some(claims) = &self.claims {
            current.record("roles", format!("{:?}", claims.authust.roles));
            if claims.authust.roles.contains(&role)
                || claims.authust.roles.contains(&UserRole::Admin)
            {
                return Ok(());
            }
        }
        tracing::warn!("Forbidden");
        Err(ErrorKind::forbidden().into())
    }
}

pub struct ApiAuth(pub SessionInfo);

#[axum::async_trait]
impl FromRequestParts<AppState> for ApiAuth {
    type Rejection = Error;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        api_auth(&parts, state).await.map(Self)
    }
}
pub struct CookieAuth(pub SessionInfo);

#[axum::async_trait]
impl FromRequestParts<AppState> for CookieAuth {
    type Rejection = Error;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        cookie_auth(&parts, state).await.map(Self)
    }
}

#[instrument(skip_all)]
async fn cookie_auth(parts: &Parts, state: &AppState) -> Result<SessionInfo, Error> {
    let cookies = CookieJar::from_headers(&parts.headers);
    let Some(session) = cookies.get(SESSION_COOKIE) else { return Err(AuthError::MissingCookie.into()) };
    let value = session.value();
    let conn = state.conn().await?;
    let stmt = conn
        .prepare_cached("select id,user_id from sessions where token = $1")
        .await?;
    let row = conn.query_opt(&stmt, &[&value]).await?;
    match row {
        Some(row) => Ok(SessionInfo {
            id: row.get("id"),
            user: row.get("user_id"),
            claims: None,
        }),
        None => Err(AuthError::InvalidSession.into()),
    }
}

#[instrument(skip_all)]
async fn api_auth(parts: &Parts, state: &AppState) -> Result<SessionInfo, Error> {
    let Some(header) = parts.headers.get("Authorization") else { return Err(AuthError::MissingHeader.into()) };
    let header = header
        .to_str()
        .map_err(|_| Error::from(AuthError::InvalidHeader))?;
    let Some(capture) = BEARER_AUTH_REGEX.captures(header) else { return Err(AuthError::InvalidHeader.into()) };
    let Some(m) = capture.get(1) else { return Err(AuthError::InvalidHeader.into()) };
    let token = m.as_str();
    let token: TokenData<Claims> =
        jsonwebtoken::decode(token, &state.auth().decoding(), &VALIDATION)?;
    Ok(SessionInfo {
        id: token.claims.sid,
        user: token.claims.sub,
        claims: Some(token.claims),
    })
}
