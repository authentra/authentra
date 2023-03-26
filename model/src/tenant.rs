#[cfg(feature = "axum")]
use axum::{
    extract::{rejection::PathRejection, Path},
    http::request::Parts,
};
use serde::Serialize;

use crate::FlowDesignation;

#[derive(Debug, Clone, Serialize)]
pub struct Tenant {
    pub uid: i32,
    pub host: String,
    pub default: bool,
    pub title: String,
    pub logo: String,
    pub favicon: String,

    pub invalidation_flow: Option<i32>,
    pub authentication_flow: Option<i32>,
    pub authorization_flow: Option<i32>,
    pub enrollment_flow: Option<i32>,
    pub recovery_flow: Option<i32>,
    pub unenrollment_flow: Option<i32>,
    pub configuration_flow: Option<i32>,
}

impl Tenant {
    pub fn get_flow(&self, designation: &FlowDesignation) -> Option<i32> {
        match designation {
            FlowDesignation::Invalidation => self.invalidation_flow,
            FlowDesignation::Authentication => self.authentication_flow,
            FlowDesignation::Authorization => self.authorization_flow,
            FlowDesignation::Enrollment => self.enrollment_flow,
            FlowDesignation::Recovery => self.enrollment_flow,
            FlowDesignation::Unenrollment => self.unenrollment_flow,
            FlowDesignation::Configuration => self.configuration_flow,
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
