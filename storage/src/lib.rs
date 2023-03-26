use async_trait::async_trait;
use deadpool_postgres::{GenericClient, Object, Pool, PoolError};
use model::{Flow, Policy, Prompt, Stage, Tenant};
use serde::Serialize;
use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::Arc,
};

pub mod flow;
pub mod policy;
pub mod prompt;
pub mod stage;
pub mod tenant;

#[derive(Debug)]
pub enum StorageError {
    Pool(deadpool_postgres::PoolError),
    Database(tokio_postgres::Error),
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            StorageError::Pool(err) => err,
            StorageError::Database(err) => err,
        })
    }
}

impl From<deadpool_postgres::PoolError> for StorageError {
    fn from(value: deadpool_postgres::PoolError) -> Self {
        Self::Pool(value)
    }
}
impl From<tokio_postgres::Error> for StorageError {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::Database(value)
    }
}

impl Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Pool(err) => Display::fmt(err, f),
            StorageError::Database(err) => Display::fmt(err, f),
        }
    }
}

#[macro_export]
macro_rules! include_sql {
    ($path:literal) => {
        include_str!(concat!("sql/", $path, ".sql"))
    };
}

#[async_trait]
pub trait ExecutorStorage: Send + Sync {
    async fn get_flow(&self, id: i32) -> StorageResult<Option<Arc<Flow>>>;
    async fn get_flow_by_slug(&self, slug: &str) -> StorageResult<Option<Arc<Flow>>>;
    async fn get_stage(&self, id: i32) -> StorageResult<Option<Arc<Stage>>>;
    async fn get_policy(&self, id: i32) -> StorageResult<Option<Arc<Policy>>>;
    async fn get_prompt(&self, id: i32) -> StorageResult<Option<Arc<Prompt>>>;
    async fn get_tenant(&self, id: i32) -> StorageResult<Option<Arc<Tenant>>>;
    async fn get_tenant_by_host(&self, host: &str) -> StorageResult<Option<Arc<Tenant>>>;

    async fn list_policies(&self) -> StorageResult<Vec<Policy>>;
    async fn list_flows(&self) -> StorageResult<Vec<Flow>>;
    async fn get_default_tenant(&self) -> StorageResult<Option<Tenant>>;
}

#[async_trait]
impl ExecutorStorage for Storage {
    async fn get_flow(&self, id: i32) -> StorageResult<Option<Arc<Flow>>> {
        self.get_flow(id).await
    }
    async fn get_flow_by_slug(&self, slug: &str) -> StorageResult<Option<Arc<Flow>>> {
        self.get_flow_by_slug(slug).await
    }
    async fn get_stage(&self, id: i32) -> StorageResult<Option<Arc<Stage>>> {
        self.get_stage(id).await
    }
    async fn get_policy(&self, id: i32) -> StorageResult<Option<Arc<Policy>>> {
        self.get_policy(id).await
    }
    async fn get_prompt(&self, id: i32) -> StorageResult<Option<Arc<Prompt>>> {
        self.get_prompt(id).await
    }
    async fn get_tenant(&self, id: i32) -> StorageResult<Option<Arc<Tenant>>> {
        self.get_tenant(id).await
    }
    async fn get_tenant_by_host(&self, host: &str) -> StorageResult<Option<Arc<Tenant>>> {
        self.get_tenant_by_host(host).await
    }
    async fn list_policies(&self) -> StorageResult<Vec<Policy>> {
        self.list_policies().await
    }
    async fn list_flows(&self) -> StorageResult<Vec<Flow>> {
        self.list_flows().await
    }
    async fn get_default_tenant(&self) -> StorageResult<Option<Tenant>> {
        self.get_default_tenant().await
    }
}

#[derive(Clone)]
pub struct Storage {
    pool: Pool,
    // connection: Arc<RwLock<Object>>,
}

impl Storage {
    pub async fn new(pool: Pool) -> Result<Self, PoolError> {
        Ok(Self {
            // connection: Arc::new(RwLock::new(pool.get().await?)),
            pool,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum EntityId {
    Flow(i32),
    Stage(i32),
    Policy(i32),
    Prompt(i32),
    Tenant(i32),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub id: i32,
    pub parent: Option<i32>,
    pub entity: EntityId,
}

impl Storage {
    pub async fn conflicts_for(
        &self,
        client: &impl GenericClient,
        entry: EntityId,
    ) -> Vec<Conflict> {
        match entry {
            EntityId::Flow(id) => self.conflicts_for_flow(client, id).await,
            EntityId::Stage(id) => self.conflicts_for_stage(client, id).await,
            EntityId::Policy(id) => self.conflicts_for_policy(client, id).await,
            EntityId::Prompt(id) => self.conflicts_for_prompt(client, id).await,
            EntityId::Tenant(id) => self.conflicts_for_tenant(client, id).await,
        }
    }

    pub async fn conflicts_for_flow(
        &self,
        _client: &impl GenericClient,
        _id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_stage(
        &self,
        _client: &impl GenericClient,
        _id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_policy(
        &self,
        _client: &impl GenericClient,
        _id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_prompt(
        &self,
        _client: &impl GenericClient,
        _id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_tenant(
        &self,
        _client: &impl GenericClient,
        _id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }

    // async fn get_client(&self) -> Result<RwLockReadGuard<Object>, PoolError> {
    //     let guard = self.connection.read().await;
    //     if guard.is_closed() {
    //         drop(guard);
    //         let mut guard = self.connection.write().await;
    //         if !guard.is_closed() {
    //             drop(guard);
    //             return Ok(self.connection.read().await);
    //         }
    //         *guard = self.pool.get().await?;
    //         drop(guard);
    //         let guard = self.connection.read().await;
    //         Ok(guard)
    //     } else {
    //         Ok(guard)
    //     }
    // }

    #[inline(always)]
    async fn get_client(&self) -> Result<Object, PoolError> {
        self.pool.get().await
    }

    pub async fn get_flow(&self, id: i32) -> StorageResult<Option<Arc<Flow>>> {
        let client = self.get_client().await?;
        arced(flow_from_db(&client, id).await)
    }
    pub async fn get_flow_by_slug(&self, slug: &str) -> StorageResult<Option<Arc<Flow>>> {
        let client = self.get_client().await?;
        arced(flow_from_db_by_slug(&client, slug).await)
    }
    pub async fn get_stage(&self, id: i32) -> StorageResult<Option<Arc<Stage>>> {
        let client = self.get_client().await?;
        arced(stage_from_db(&client, id).await)
    }
    pub async fn get_policy(&self, id: i32) -> StorageResult<Option<Arc<Policy>>> {
        let client = self.get_client().await?;
        arced(policy_from_db(&client, id).await)
    }
    pub async fn get_prompt(&self, id: i32) -> StorageResult<Option<Arc<Prompt>>> {
        let client = self.get_client().await?;
        arced(prompt_from_db(&client, id).await)
    }
    pub async fn get_tenant(&self, id: i32) -> StorageResult<Option<Arc<Tenant>>> {
        let client = self.get_client().await?;
        arced(tenant_from_db(&client, id).await)
    }
    pub async fn get_tenant_by_host(&self, host: &str) -> StorageResult<Option<Arc<Tenant>>> {
        let client = self.get_client().await?;
        arced(tenant_from_db_by_host(&client, host).await)
    }

    pub async fn list_policies(&self) -> StorageResult<Vec<Policy>> {
        let client = self.get_client().await?;
        list_policies_from_db(&client).await
    }
    pub async fn list_flows(&self) -> StorageResult<Vec<Flow>> {
        let client = self.get_client().await?;
        list_flows_from_db(&client).await
    }

    async fn get_default_tenant(&self) -> StorageResult<Option<Tenant>> {
        let client = self.get_client().await?;
        default_tenant_from_db(&client).await
    }
}

fn arced<T, E>(v: Result<Option<T>, E>) -> Result<Option<Arc<T>>, E> {
    match v {
        Ok(opt) => Ok(opt.map(Arc::new)),
        Err(err) => Err(err),
    }
}

async fn flow_from_db(client: &Object, id: i32) -> StorageResult<Option<Flow>> {
    let statement = client.prepare_cached(include_sql!("flow/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::flow::from_row(client, row).await?)),
        None => Ok(None),
    }
}
async fn flow_from_db_by_slug(client: &Object, slug: &str) -> StorageResult<Option<Flow>> {
    let statement = client.prepare_cached(include_sql!("flow/by-slug")).await?;
    match client.query_opt(&statement, &[&slug]).await? {
        Some(row) => Ok(Some(crate::flow::from_row(client, row).await?)),
        None => Ok(None),
    }
}
async fn stage_from_db(client: &Object, id: i32) -> StorageResult<Option<Stage>> {
    let statement = client.prepare_cached(include_sql!("stage/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::stage::from_row(client, row).await?)),
        None => Ok(None),
    }
}
async fn policy_from_db(client: &Object, id: i32) -> StorageResult<Option<Policy>> {
    let statement = client.prepare_cached(include_sql!("policy/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::policy::from_row(row)?)),
        None => Ok(None),
    }
}
async fn prompt_from_db(client: &Object, id: i32) -> StorageResult<Option<Prompt>> {
    let statement = client.prepare_cached(include_sql!("prompt/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::prompt::from_row(row))),
        None => Ok(None),
    }
}
async fn tenant_from_db(client: &Object, id: i32) -> StorageResult<Option<Tenant>> {
    let statement = client.prepare_cached(include_sql!("tenant/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::tenant::from_row(row))),
        None => Ok(None),
    }
}
async fn default_tenant_from_db(client: &Object) -> StorageResult<Option<Tenant>> {
    let statement = client
        .prepare_cached(include_sql!("tenant/get-default"))
        .await?;
    match client.query_opt(&statement, &[]).await? {
        Some(row) => Ok(Some(crate::tenant::from_row(row))),
        None => Ok(None),
    }
}
async fn tenant_from_db_by_host(client: &Object, host: &str) -> StorageResult<Option<Tenant>> {
    let statement = client
        .prepare_cached(include_sql!("tenant/by-host"))
        .await?;
    match client.query_opt(&statement, &[&host]).await? {
        Some(row) => Ok(Some(crate::tenant::from_row(row))),
        None => Ok(None),
    }
}

async fn list_policies_from_db(client: &Object) -> StorageResult<Vec<Policy>> {
    let statement = client
        .prepare_cached(include_sql!("policy/list-all"))
        .await?;
    client
        .query(&statement, &[])
        .await?
        .into_iter()
        .map(crate::policy::from_row)
        .collect()
}

async fn list_flows_from_db(client: &Object) -> StorageResult<Vec<Flow>> {
    let statement = client.prepare_cached(include_sql!("flow/list-all")).await?;
    let iter = client.query(&statement, &[]).await?.into_iter();
    let mut flows = Vec::new();
    for row in iter {
        let flow = crate::flow::from_row(client, row).await?;
        flows.push(flow);
    }
    Ok(flows)
}
