use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{rejection::HostRejection, FromRequestParts, Host, RawQuery, State},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use derive_more::From;
use http::{header::LOCATION, request::Parts, StatusCode};
use model::{FlowDesignation, Tenant};
use once_cell::sync::Lazy;
use storage::StorageError;

use crate::{api::ApiError, SharedState};

pub fn setup_flow_router() -> Router<SharedState> {
    Router::new().route("/:flow_designation", get(tenant_flow_redirect))
}

static INTERFACE_BASE_URI: Lazy<&'static str> = Lazy::new(base_uri);

fn base_uri() -> &'static str {
    let env = std::env::var("INTERFACE_BASE_URI").ok();
    match env {
        Some(v) => Box::leak(v.into_boxed_str()),
        None => "",
    }
}

pub async fn tenant_flow_redirect(
    tenant: Arc<Tenant>,
    designation: FlowDesignation,
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiError> {
    if let Some(flow) = tenant.get_flow(&designation) {
        if let Some(flow) = state.storage().get_flow(flow).await? {
            let uri = match query {
                Some(query) => format!("{}/flow/{}?{query}", *INTERFACE_BASE_URI, flow.slug),
                None => format!("{}/flow/{}", *INTERFACE_BASE_URI, flow.slug),
            };
            return Ok((StatusCode::FOUND, [(LOCATION, uri)]).into_response());
        }
    }
    Ok(StatusCode::NOT_FOUND.into_response())
}

#[derive(From)]
pub enum TenantRejection {
    Host(HostRejection),
    Internal(StorageError),
    NotFound,
}

impl IntoResponse for TenantRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            TenantRejection::Host(host) => host.into_response(),
            TenantRejection::NotFound => StatusCode::NOT_FOUND.into_response(),
            TenantRejection::Internal(err) => ApiError::from(err).into_response(),
        }
    }
}

#[async_trait]
impl FromRequestParts<SharedState> for Arc<Tenant> {
    type Rejection = TenantRejection;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {
        let host = Host::from_request_parts(parts, state).await?;
        if let Some(tenant) = state.storage().get_tenant_by_host(host.0.as_str()).await? {
            return Ok(tenant);
        }
        state.defaults().tenant().ok_or(TenantRejection::NotFound)
    }
}
