use std::{fmt::Debug, str::Chars};

use async_trait::async_trait;
use http::{
    header::{self, HOST},
    uri::Scheme,
    Method, StatusCode, Uri,
};
use poem::{
    web::cookie::{Cookie, CookieJar, SameSite},
    Endpoint, IntoResponse, Middleware, Request, Response,
};
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

use super::CSRF_HEADER;

const CSRF_COOKIE_NAME: &'static str = "csrf-token";
const CSRF_SECRET_LENGTH: usize = 32;
const CSRF_TOKEN_LENGTH: usize = CSRF_SECRET_LENGTH * 2;

const ALPHANUMERIC_CHARS: [char; 62] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];
const ALPHANUMERIC_RANGE: usize = ALPHANUMERIC_CHARS.len();

#[derive(Debug, Clone)]
pub struct CsrfLayer {
    allowed_hosts: Vec<String>,
}

impl CsrfLayer {
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_hosts: allowed_origins,
        }
    }
}

impl<E: Endpoint> Middleware<E> for CsrfLayer {
    type Output = CsrfEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CsrfEndpoint {
            allowed_hosts: self.allowed_hosts.clone(),
            allowed_hosts_wildcard: self.allowed_hosts.contains(&"*".to_owned()),
            inner: ep,
        }
    }
}

pub struct CsrfEndpoint<E: Endpoint> {
    allowed_hosts: Vec<String>,
    allowed_hosts_wildcard: bool,
    inner: E,
}

#[async_trait]
impl<E: Endpoint> Endpoint for CsrfEndpoint<E> {
    type Output = poem::Response;
    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        if let Err(err) = check_csrf(&req, self.allowed_hosts_wildcard, &self.allowed_hosts) {
            return Ok(err);
        }
        self.inner.call(req).await.map(IntoResponse::into_response)
    }
}

fn check_csrf(
    req: &Request,
    allowed_hosts_wildcard: bool,
    allowed_hosts: &Vec<String>,
) -> Result<(), Response> {
    if matches!(
        req.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS | &Method::TRACE
    ) {
        if let Err(err) = check_cookie(&req, true) {
            if err == CsrfCookieError::ValidationError {
                return Err(csrf_error_response());
            }
        }
        return Ok(());
    }
    let uri = req.uri();
    let host = req
        .headers()
        .get(HOST)
        .ok_or((StatusCode::BAD_REQUEST, "Misssing host header").into_response())
        .and_then(|v| {
            v.to_str()
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Host header").into_response())
        })?;

    if let Some(origin) = req.headers().get(header::ORIGIN) {
        let origin = origin
            .to_str()
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
        let host = format!("{}://{host}", if is_secure(uri) { "https" } else { "http" });
        if !(host == origin || allowed_hosts_wildcard || allowed_hosts.contains(&host)) {
            return Err(csrf_error_response());
        }
    }
    if let Err(err) = check_cookie(&req, true) {
        if err == CsrfCookieError::ValidationError {
            return Err(csrf_error_response());
        }
    }
    Ok(())
}

#[derive(PartialEq)]
enum CsrfCookieError {
    MissingCookie,
    InvalidHeader,
    ValidationError,
}

fn add_csrf_cookie(cookies: &CookieJar) {
    let mut cookie = Cookie::new(CSRF_COOKIE_NAME, generate_csrf());
    cookie.set_same_site(SameSite::Lax);
    cookies.add(cookie);
}

fn check_cookie(req: &Request, add: bool) -> Result<(), CsrfCookieError> {
    let cookies = req.cookie();
    let Some(cookie) = cookies.get(CSRF_COOKIE_NAME) else {
        if add {
            add_csrf_cookie(cookies);
        }
        return Err(CsrfCookieError::MissingCookie);
    };
    let Some(header) = req.headers().get(CSRF_HEADER) else { return Err(CsrfCookieError::InvalidHeader) };
    let Ok(str) = header.to_str() else { return Err(CsrfCookieError::InvalidHeader) };
    if validate_token(str, cookie.value_str()) {
        Ok(())
    } else {
        Err(CsrfCookieError::ValidationError)
    }
}

fn csrf_error_response() -> Response {
    (StatusCode::FORBIDDEN, "CSRF Error").into_response()
}

fn is_secure(uri: &Uri) -> bool {
    uri.scheme()
        .map(|scheme| scheme == &Scheme::HTTPS)
        .unwrap_or(false)
}

pub(self) fn generate_csrf() -> String {
    let mut rng = thread_rng();
    Alphanumeric.sample_string(&mut rng, CSRF_SECRET_LENGTH)
}

#[allow(unused)]
pub(self) fn mask_csrf(secret: &str) -> String {
    let mask_str = generate_csrf();
    let out = mask_op(mask_str.chars(), secret.chars(), true);
    mask_str + &out
}

fn mask_op(mask: Chars, chars: Chars, add: bool) -> String {
    let mask = mask.map(|c| {
        ALPHANUMERIC_CHARS
            .iter()
            .position(|&v| v == c)
            .expect("Not alphanumeric char")
    });
    let pairs = chars
        .map(|c| {
            ALPHANUMERIC_CHARS
                .iter()
                .position(|&v| v == c)
                .expect("Not alphanumeric char")
        })
        .zip(mask);
    if add {
        pairs
            .map(|(x, y)| ALPHANUMERIC_CHARS[(x + y) % ALPHANUMERIC_RANGE])
            .collect()
    } else {
        pairs
            .map(|(x, y)| ALPHANUMERIC_CHARS[overflowing_sub_chars(x, y)])
            .collect()
    }
}

fn overflowing_sub_chars(x: usize, y: usize) -> usize {
    let (value, overflowed) = x.overflowing_sub(y);
    if overflowed {
        let value = usize::MAX - value;
        ALPHANUMERIC_RANGE - 1 - value
    } else {
        value
    }
}

pub(self) fn unmask_csrf(token: &str) -> String {
    let mask = &token[..CSRF_SECRET_LENGTH];
    let token = &token[CSRF_SECRET_LENGTH..];
    mask_op(mask.chars(), token.chars(), false)
}

pub fn validate_token(request_token: &str, secret: &str) -> bool {
    if request_token.len() == CSRF_TOKEN_LENGTH {
        let token = &unmask_csrf(request_token);
        token == secret
    } else {
        request_token == secret
    }
}

#[cfg(test)]
mod test {
    use crate::api::csrf::unmask_csrf;

    use super::{generate_csrf, mask_csrf, validate_token};

    #[test]
    fn test_mask() {
        let secret = generate_csrf();
        let masked = mask_csrf(&secret);
        assert_eq!(secret, unmask_csrf(&masked));
    }

    #[test]
    fn test_secret_validate() {
        let secret = generate_csrf();
        assert!(validate_token(&secret, &secret), "Secret validation failed");
    }

    #[test]
    fn test_token_validation() {
        let secret = generate_csrf();
        let masked = mask_csrf(&secret);
        assert!(validate_token(&masked, &secret), "Token validation failed");
    }
}
