#[cfg(feature = "axum")]
use axum::{
    extract::{rejection::PathRejection, Path},
    http::request::Parts,
};
use datacache::DataRef;
use serde::Serialize;

use crate::{Flow, FlowDesignation};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "datacache", derive(datacache::DataMarker))]
pub struct Tenant {
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub uid: i32,
    #[cfg_attr(feature = "datacache", datacache(queryable))]
    pub host: String,
    pub default: bool,
    pub title: String,
    pub logo: String,
    pub favicon: String,

    pub invalidation_flow: Option<DataRef<Flow>>,
    pub authentication_flow: Option<DataRef<Flow>>,
    pub authorization_flow: Option<DataRef<Flow>>,
    pub enrollment_flow: Option<DataRef<Flow>>,
    pub recovery_flow: Option<DataRef<Flow>>,
    pub unenrollment_flow: Option<DataRef<Flow>>,
    pub configuration_flow: Option<DataRef<Flow>>,
}

impl Tenant {
    pub fn get_flow(&self, designation: &FlowDesignation) -> Option<DataRef<Flow>> {
        match designation {
            FlowDesignation::Invalidation => self.invalidation_flow.clone(),
            FlowDesignation::Authentication => self.authentication_flow.clone(),
            FlowDesignation::Authorization => self.authorization_flow.clone(),
            FlowDesignation::Enrollment => self.enrollment_flow.clone(),
            FlowDesignation::Recovery => self.enrollment_flow.clone(),
            FlowDesignation::Unenrollment => self.unenrollment_flow.clone(),
            FlowDesignation::Configuration => self.configuration_flow.clone(),
        }
    }
}

#[cfg(feature = "axum")]
#[derive(serde::Deserialize)]
pub struct FlowDesignationParam {
    flow_designation: FlowDesignation,
}

#[cfg(feature = "axum")]
#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for FlowDesignation
where
    S: Send + Sync,
{
    type Rejection = PathRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path: Path<FlowDesignationParam> = Path::from_request_parts(parts, state).await?;
        Ok(path.0.flow_designation)
    }
}
