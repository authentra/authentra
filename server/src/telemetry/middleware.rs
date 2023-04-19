use std::{borrow::Cow, time::Duration};

use axum::{
    extract::MatchedPath,
    http::{header, uri::Scheme, Method, Request, Response, Version},
};
use opentelemetry::sdk::trace::{IdGenerator, RandomIdGenerator};
use tower_http::{
    classify::ServerErrorsFailureClass,
    trace::{MakeSpan, OnBodyChunk, OnEos, OnFailure, OnRequest, OnResponse, TraceLayer},
};
use tracing::{field::Empty, Span};

use crate::api::Error;

pub fn new() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    OtelMakeSpan,
    OtelOnRequest,
    OtelOnResponse,
    OtelOnBodyChunk,
    OtelOnEos,
    OtelOnFailure,
> {
    TraceLayer::new_for_http()
        .make_span_with(OtelMakeSpan)
        .on_request(OtelOnRequest)
        .on_response(OtelOnResponse)
        .on_body_chunk(OtelOnBodyChunk)
        .on_eos(OtelOnEos)
        .on_failure(OtelOnFailure)
}

#[derive(Clone)]
pub struct OtelMakeSpan;

impl<B> MakeSpan<B> for OtelMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let user_agent = request
            .headers()
            .get(header::USER_AGENT)
            .map_or("", |v| v.to_str().unwrap_or(""));
        let host = request
            .headers()
            .get(header::HOST)
            .map_or("", |v| v.to_str().unwrap_or(""));
        let scheme = request.uri().scheme().map_or("http", http_scheme);
        let route = request
            .extensions()
            .get::<MatchedPath>()
            .map(|path| path.as_str())
            .unwrap();
        let target = request
            .uri()
            .path_and_query()
            .map_or("", |target| target.as_str());
        let method = http_method(request.method());
        let flavor = http_flavor(request.version());
        let name = format!("{method} {route}");
        let trace_id = RandomIdGenerator::default().new_trace_id();
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

#[derive(Clone)]
pub struct OtelOnRequest;

impl<B> OnRequest<B> for OtelOnRequest {
    fn on_request(&mut self, _request: &Request<B>, _span: &Span) {}
}

#[derive(Clone)]
pub struct OtelOnResponse;

impl<B> OnResponse<B> for OtelOnResponse {
    fn on_response(self, response: &Response<B>, _latency: Duration, span: &Span) {
        span.record("http.status_code", response.status().as_u16());
        span.record("otel.status_code", "OK");
        if let Some(error) = response.extensions().get::<Error>() {
            span.record("exception.message", format!("{}", error.kind()));
            if let Some(trace) = error.trace() {
                span.record("exception.stacktrace", format!("{:?}", trace));
            }
        }
    }
}

#[derive(Clone)]
pub struct OtelOnBodyChunk;

impl<B> OnBodyChunk<B> for OtelOnBodyChunk {
    fn on_body_chunk(&mut self, _chunk: &B, _latency: Duration, _span: &Span) {}
}

#[derive(Clone)]
pub struct OtelOnEos;

impl OnEos for OtelOnEos {
    fn on_eos(
        self,
        _trailers: Option<&axum::http::HeaderMap>,
        _stream_duration: Duration,
        _span: &Span,
    ) {
    }
}

#[derive(Clone)]
pub struct OtelOnFailure;

impl OnFailure<ServerErrorsFailureClass> for OtelOnFailure {
    fn on_failure(
        &mut self,
        classification: ServerErrorsFailureClass,
        _latency: Duration,
        span: &Span,
    ) {
        match classification {
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
