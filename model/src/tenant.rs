#[cfg(feature = "axum")]
use axum::{
    extract::{rejection::PathRejection, Path},
    http::request::Parts,
};
use serde::Serialize;

use crate::{Flow, FlowDesignation, Reference};

#[derive(Debug, Clone, Serialize)]
pub struct Tenant {
    pub uid: i32,
    pub host: String,
    pub default: bool,
    pub title: String,
    pub logo: String,
    pub favicon: String,

    pub invalidation_flow: Option<Reference<Flow>>,
    pub authentication_flow: Option<Reference<Flow>>,
    pub authorization_flow: Option<Reference<Flow>>,
    pub enrollment_flow: Option<Reference<Flow>>,
    pub recovery_flow: Option<Reference<Flow>>,
    pub unenrollment_flow: Option<Reference<Flow>>,
    pub configuration_flow: Option<Reference<Flow>>,
}

impl Tenant {
    pub fn get_flow(&self, designation: &FlowDesignation) -> Option<Reference<Flow>> {
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

#[cfg(feature = "sqlx")]
impl From<PgTenant> for Tenant {
    fn from(value: PgTenant) -> Self {
        Tenant {
            uid: value.uid,
            host: value.host,
            default: value.default,
            title: value.title,
            logo: value.logo,
            favicon: value.favicon,
            invalidation_flow: value.invalidation_flow.map(Reference::new_uid),
            authentication_flow: value.authentication_flow.map(Reference::new_uid),
            authorization_flow: value.authorization_flow.map(Reference::new_uid),
            enrollment_flow: value.enrollment_flow.map(Reference::new_uid),
            recovery_flow: value.recovery_flow.map(Reference::new_uid),
            unenrollment_flow: value.unenrollment_flow.map(Reference::new_uid),
            configuration_flow: value.configuration_flow.map(Reference::new_uid),
        }
    }
}

#[cfg(feature = "sqlx")]
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PgTenant {
    pub uid: i32,
    pub host: String,
    #[sqlx(rename = "is_default")]
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
