use axum::{
    http::{header, Request},
    response::{IntoResponse, Response},
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use futures::future::BoxFuture;
use jsonwebtoken::{
    errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header as JwtHeader, TokenData,
    Validation,
};
use once_cell::sync::Lazy;
use rand::{CryptoRng, RngCore};

use regex::Regex;
use serde::{de::DeserializeOwned, Serialize};

use tower::Service;
use tower_cookies::{Cookie, Cookies};
use tracing::instrument;

use crate::api::{Error, ErrorKind};

use super::{AuthExtension, AuthState, Claims};

static BEARER_AUTH_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("^Bearer ([a-zA-Z0-9-_=]{16,})$").unwrap());

static VALIDATION: Lazy<Validation> = Lazy::new(|| {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["iss"]);
    validation.set_issuer(&["authust"]);
    validation
});

#[derive(Clone)]
pub struct AuthService<S> {
    pub(crate) state: AuthState,
    pub inner: S,
}

impl<S, B> Service<Request<B>> for AuthService<S>
where
    B: Send + 'static,
    S: Service<Request<B>> + Send + Clone + 'static,
    S::Response: IntoResponse,
    S::Future: Send,
{
    type Response = Response;

    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, S::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let clone = self.inner.clone();
        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let state = self.state.clone();
        Box::pin(async move {
            if let Err(error) = handle_auth(state, &mut req).await {
                return Ok(error.into_response());
            }
            inner.call(req).await.map(IntoResponse::into_response)
        })
    }
}

#[instrument(skip(state, req))]
async fn handle_auth<B>(state: AuthState, req: &mut Request<B>) -> Result<(), Error> {
    let cookies: &Cookies = req
        .extensions()
        .get()
        .expect("Cookie middleware must come before auth middleware");
    let token = find_jwt(cookies, &req)?;
    let extension = match token {
        Some(token) => {
            let data = Session::decode_jwt::<Claims>(state.decoding(), token.as_str()).unwrap();
            AuthExtension {
                claims: data.claims,
                is_generated: false,
            }
        }
        None => {
            let claims = state.with_rng(|rng| create_claims(rng)).await;
            let cookie = session_cookie(state.encoding(), &claims)?;
            cookies.add(cookie);
            AuthExtension {
                claims,
                is_generated: true,
            }
        }
    };
    req.extensions_mut().insert(extension);
    Ok(())
}

fn session_cookie(encoding: &EncodingKey, claims: &Claims) -> Result<Cookie<'static>, JwtError> {
    let value = Session::encode_jwt(encoding, claims)?;
    let mut cookie = Cookie::named("session");
    cookie.set_http_only(true);
    cookie.set_value(value);
    Ok(cookie)
}

#[inline(always)]
fn find_jwt<B>(cookies: &Cookies, req: &Request<B>) -> Result<Option<String>, Error> {
    if let Some(header) = req.headers().get(header::AUTHORIZATION) {
        let header = header.to_str().map_err(|_| ErrorKind::HeaderNotAscii {
            header_name: header::AUTHORIZATION.as_str(),
        })?;
        let Some(captures) = BEARER_AUTH_REGEX.captures(header) else { return Err(ErrorKind::InvalidAuthorizationHeader.into()) };
        let Some(m) = captures.get(1) else { return Err(ErrorKind::InvalidAuthorizationHeader.into()) };
        return Ok(Some(m.as_str().to_owned()));
    }
    if let Some(cookie) = cookies.get("session") {
        return Ok(Some(cookie.value().to_owned()));
    }
    Ok(None)
}

pub struct Session;

impl Session {
    pub fn encode_jwt<T: Serialize>(
        encoding: &EncodingKey,
        claims: &T,
    ) -> Result<String, JwtError> {
        jsonwebtoken::encode(
            &JwtHeader::new(jsonwebtoken::Algorithm::HS256),
            claims,
            encoding,
        )
    }
    pub fn decode_jwt<T: DeserializeOwned>(
        decoding: &DecodingKey,
        token: &str,
    ) -> Result<TokenData<T>, JwtError> {
        jsonwebtoken::decode(token, decoding, &VALIDATION)
    }
}

fn generate_session_id<R: RngCore + CryptoRng>(rng: &mut R) -> String {
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_device_id<R: RngCore + CryptoRng>(rng: &mut R) -> String {
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

pub fn create_claims<R: RngCore + CryptoRng>(rng: &mut R) -> Claims {
    Claims {
        iss: "authust".into(),
        sid: generate_session_id(rng),
        sub: None,
    }
}
