use std::task::{ready, Poll};
use std::{borrow::Cow, time::Duration};

use axum::extract::{FromRequestParts, Host, MatchedPath};
use axum::response::{IntoResponse, Response as AxumResponse};
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use http::request::Parts;
use http::{header, uri::Scheme, Method, Request, Response, Version};
use once_cell::unsync::Lazy;
use opentelemetry::sdk::trace::{IdGenerator, RandomIdGenerator};
use tower::{Layer, Service};
use tower_http::{
    classify::{ServerErrorsFailureClass, SharedClassifier},
    trace::{MakeSpan, OnBodyChunk, OnEos, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::{field::Empty, Span};

const ID_GENERATOR: Lazy<RandomIdGenerator> = Lazy::new(RandomIdGenerator::default);

#[derive(Clone)]
pub struct ExtensionLayer;

impl<S> Layer<S> for ExtensionLayer {
    type Service = ExtensionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ExtensionService { inner }
    }
}

#[derive(Clone)]
pub struct ExtensionService<S> {
    inner: S,
}

impl<S, Body> Service<Request<Body>> for ExtensionService<S>
where
    S: Service<Request<Body>> + Clone,
    S::Response: IntoResponse,
    Body: Send + 'static,
{
    type Response = AxumResponse;

    type Error = S::Error;

    type Future = ResponseFuture<Body, S>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let extract = extract_requests(req.into_parts()).boxed();
        let not_ready_inner = self.inner.clone();
        let service = std::mem::replace(&mut self.inner, not_ready_inner);
        ResponseFuture {
            state: State::Extracting(extract),
            inner: Some(service),
        }
    }
}
async fn extract_requests<B>((mut parts, body): (Parts, B)) -> Result<(Parts, B), AxumResponse> {
    let host = Host::from_request_parts(&mut parts, &())
        .await
        .map_err(IntoResponse::into_response)?;
    parts.extensions.insert(host);
    Ok((parts, body))
}

#[pin_project::pin_project]
pub struct ResponseFuture<B, T: Service<Request<B>>> {
    #[pin]
    state: State<B, T>,
    inner: Option<T>,
}

#[pin_project::pin_project(project = StateProj)]
pub enum State<B, T: Service<Request<B>>> {
    Extracting(BoxFuture<'static, Result<(Parts, B), AxumResponse>>),
    Future(#[pin] T::Future),
}

impl<B, T: Service<Request<B>>> Future for ResponseFuture<B, T>
where
    T::Response: IntoResponse,
{
    type Output = Result<AxumResponse, T::Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        loop {
            let mut this = self.as_mut().project();
            let new_state = match this.state.as_mut().project() {
                StateProj::Extracting(future) => {
                    let res = ready!(future.as_mut().poll(cx));
                    match res {
                        Ok((parts, body)) => {
                            let request = Request::from_parts(parts, body);
                            let mut svc = std::mem::replace(this.inner, None)
                                .expect("Already replaced inner service with none");
                            State::Future(svc.call(request))
                        }
                        Err(err) => return Poll::Ready(Ok(err)),
                    }
                }
                StateProj::Future(future) => {
                    return future
                        .poll(cx)
                        .map(|result| result.map(IntoResponse::into_response))
                }
            };
            this.state.set(new_state);
        }
    }
}

pub fn otel_layer() -> TraceLayer<
    SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    OtelData,
    OtelData,
    OtelData,
    OtelData,
    OtelData,
    OtelData,
> {
    TraceLayer::new_for_http()
        .make_span_with(OtelData)
        .on_request(OtelData)
        .on_response(OtelData)
        .on_body_chunk(OtelData)
        .on_eos(OtelData)
        .on_failure(OtelData)
}

pub struct OtelData;

impl Clone for OtelData {
    fn clone(&self) -> Self {
        Self
    }
}

impl<B> MakeSpan<B> for OtelData {
    fn make_span(&mut self, req: &Request<B>) -> Span {
        let headers = req.headers();
        let user_agent = headers
            .get(header::USER_AGENT)
            .map_or("", |hdr| hdr.to_str().unwrap_or(""));
        let host = req
            .extensions()
            .get::<Host>()
            .map(|host| host.0.as_str())
            .unwrap_or("");
        let scheme = req.uri().scheme().map_or_else(|| "http", http_scheme);
        let route = req
            .extensions()
            .get::<MatchedPath>()
            .map(|path| path.as_str())
            .unwrap_or_else(|| req.uri().path());
        let target = req
            .uri()
            .path_and_query()
            .map_or("", |target| target.as_str());
        let method = http_method(req.method());
        let flavor = http_flavor(req.version());
        let name = format!("{method} {route}");
        let trace_id = ID_GENERATOR.new_trace_id();
        let span = tracing::info_span!(
            "Http Request",
            otel.name = name,
            http.method = %method,
            http.flavor = %flavor,
            http.target = target,
            http.host = host,
            http.scheme = scheme,
            http.route = route,
            http.status_code = Empty,
            http.user_agent = user_agent,
            otel.kind = "server",
            otel.status_code = Empty,
            trace_id = %trace_id,
        );
        span
    }
}

fn http_scheme(scheme: &Scheme) -> &str {
    if scheme == &Scheme::HTTP {
        "http"
    } else if scheme == &Scheme::HTTPS {
        "https"
    } else {
        scheme.as_str()
    }
}
fn http_flavor(version: Version) -> Cow<'static, str> {
    match version {
        Version::HTTP_09 => "0.9".into(),
        Version::HTTP_10 => "1.0".into(),
        Version::HTTP_11 => "1.1".into(),
        Version::HTTP_2 => "2.0".into(),
        Version::HTTP_3 => "3.0".into(),
        other => format!("{other:?}").into(),
    }
}

fn http_method(method: &Method) -> Cow<'static, str> {
    match method {
        &Method::GET => "GET".into(),
        &Method::HEAD => "HEAD".into(),
        &Method::POST => "POST".into(),
        &Method::PUT => "PUT".into(),
        &Method::DELETE => "DELETE".into(),
        &Method::CONNECT => "CONNECT".into(),
        &Method::OPTIONS => "OPTIONS".into(),
        &Method::TRACE => "TRACE".into(),
        &Method::PATCH => "PATCH".into(),
        other => other.to_string().into(),
    }
}

impl<B> OnRequest<B> for OtelData {
    fn on_request(&mut self, _request: &Request<B>, _span: &Span) {}
}

impl<B> OnResponse<B> for OtelData {
    fn on_response(self, response: &Response<B>, _latency: Duration, span: &Span) {
        span.record("http.status_code", response.status().as_u16());
        span.record("otel.status_code", "OK");
    }
}

impl<B> OnBodyChunk<B> for OtelData {
    fn on_body_chunk(&mut self, _chunk: &B, _latency: Duration, _span: &Span) {}
}

impl OnEos for OtelData {
    fn on_eos(self, _trailers: Option<&http::HeaderMap>, _stream_duration: Duration, _span: &Span) {
    }
}

impl OnFailure<ServerErrorsFailureClass> for OtelData {
    fn on_failure(
        &mut self,
        failure_classification: ServerErrorsFailureClass,
        _latency: Duration,
        span: &Span,
    ) {
        match failure_classification {
            ServerErrorsFailureClass::StatusCode(status) => {
                if !status.is_server_error() {
                    return;
                }
            }
            _ => return,
        };
        span.record("otel.status_code", "ERROR");
    }
}
