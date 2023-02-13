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
use model::{FlowDesignation, Reference, Tenant};

use crate::{flow_storage::ReferenceLookup, SharedState};

pub fn setup_flow_router() -> Router<SharedState> {
    Router::new().route("/:flow_designation", get(tenant_flow_redirect))
}

const INTERFACE_BASE_URI: &str = base_uri();

const fn base_uri() -> &'static str {
    let env = option_env!("INTERFACE_BASE_URI");
    match env {
        Some(v) => v,
        None => "",
    }
}

pub async fn tenant_flow_redirect(
    tenant: Arc<Tenant>,
    designation: FlowDesignation,
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
) -> Response {
    if let Some(flow) = tenant.get_flow(&designation) {
        if let Some(flow) = state.storage().lookup_reference(&flow).await {
            let uri = match query {
                Some(query) => format!("{INTERFACE_BASE_URI}/flow/{}?{query}", flow.slug),
                None => format!("{INTERFACE_BASE_URI}/flow/{}", flow.slug),
            };
            return (StatusCode::FOUND, [(LOCATION, uri)]).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

#[derive(From)]
pub enum TenantRejection {
    Host(HostRejection),
    NotFound,
}

impl IntoResponse for TenantRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            TenantRejection::Host(host) => host.into_response(),
            TenantRejection::NotFound => StatusCode::NOT_FOUND.into_response(),
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
        let reference: Reference<Tenant> = Reference::new_slug(host.0);
        if let Some(tenant) = state.storage().lookup_reference(&reference).await {
            return Ok(tenant);
        }
        state.defaults().tenant().ok_or(TenantRejection::NotFound)
    }
}
