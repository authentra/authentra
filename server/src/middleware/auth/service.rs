use axum::{
    http::Request,
    response::{IntoResponse, Response},
};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use futures::future::BoxFuture;
use jsonwebtoken::{
    errors::Error as JwtError, DecodingKey, EncodingKey, Header as JwtHeader, TokenData, Validation,
};
use rand::{CryptoRng, RngCore};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use tower::Service;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::api::Error;

use super::AuthState;

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
            if let Err(error) = Self::handle_auth(state, &mut req).await {
                return Ok(error.into_response());
            }
            inner.call(req).await.map(IntoResponse::into_response)
        })
    }
}

impl<S> AuthService<S> {
    async fn handle_auth<B>(state: AuthState, req: &mut Request<B>) -> Result<(), Error> {
        let cookies: &Cookies = req
            .extensions()
            .get()
            .expect("Cookie middleware must come before auth middleware");
        let data = Session::decode_jwt::<Claims>(state.decoding(), "").unwrap();
        todo!()
    }
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
        jsonwebtoken::decode(token, decoding, &Validation::default())
    }
}

fn generate_session_id<R: RngCore + CryptoRng>(rng: &mut R) -> String {
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    iss: String,
    sid: String,
    sub: Option<Uuid>,
}
