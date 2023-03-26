use std::sync::Arc;

use deadpool_postgres::{GenericClient, Object, Pool, PoolError};
use model::Tenant;
use storage::Storage;

use crate::{
    api::AuthServiceData,
    executor::FlowExecutor,
    service::{policy::PolicyService, user::UserService},
};

#[derive(Clone)]
pub struct SharedState(pub(super) Arc<InternalSharedState>);

impl SharedState {
    pub fn users(&self) -> &UserService {
        &self.0.users
    }

    pub fn executor(&self) -> &FlowExecutor {
        &self.0.executor
    }

    pub fn auth_data(&self) -> &AuthServiceData {
        &self.0.auth_data
    }
    pub fn storage(&self) -> &Storage {
        &self.0.storage
    }

    pub fn defaults(&self) -> &Defaults {
        self.0.defaults.as_ref()
    }
    pub fn policies(&self) -> &PolicyService {
        &self.0.policies
    }
}

pub(super) struct InternalSharedState {
    pub(super) users: UserService,
    pub(super) executor: FlowExecutor,
    pub(super) auth_data: AuthServiceData,
    pub(super) storage: Storage,
    pub(super) defaults: Arc<Defaults>,
    pub(super) policies: PolicyService,
}

pub struct Defaults {
    pool: Pool,
    tenant: Option<Arc<Tenant>>,
}

impl Defaults {
    pub fn tenant(&self) -> Option<Arc<Tenant>> {
        self.tenant.clone()
    }
    pub fn pool(&self) -> Pool {
        self.pool.clone()
    }
    pub async fn connection(&self) -> Result<Object, PoolError> {
        self.pool.get().await
    }
}

impl Defaults {
    pub async fn new(storage: &Storage, pool: Pool) -> Self {
        let default = Self::find_default(
            storage,
            &pool.get().await.expect("Failed to get connection"),
        )
        .await;
        Defaults {
            pool,
            tenant: default,
        }
    }

    async fn find_default(storage: &Storage, client: &impl GenericClient) -> Option<Arc<Tenant>> {
        let statement = client
            .prepare_cached("select uid from tenants where is_default = true")
            .await
            .expect("Failed to prepare statement");
        let Some(row) = client
            .query_opt(&statement, &[])
            .await
            .expect("Failed to execute statement") else { return None };

        storage
            .get_tenant(row.get("uid"))
            .await
            .expect("An error occurred while loading tenant")
    }
}
