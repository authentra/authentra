use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use deadpool_postgres::{GenericClient, Object, Pool, PoolError};
use futures::{future::LocalBoxFuture, Future, FutureExt};
use model::{Flow, Policy, Prompt, Stage, Tenant};
use moka::future::Cache;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{include_sql, StorageError};

#[async_trait(?Send)]
pub trait ExecutorStorage {
    async fn get_flow(&self, id: i32) -> StorageResult<Option<Arc<Flow>>>;
    async fn get_stage(&self, id: i32) -> StorageResult<Option<Arc<Stage>>>;
    async fn get_policy(&self, id: i32) -> StorageResult<Option<Arc<Policy>>>;
    async fn get_prompt(&self, id: i32) -> StorageResult<Option<Arc<Prompt>>>;
    async fn get_tenant(&self, id: i32) -> StorageResult<Option<Arc<Tenant>>>;
}

#[async_trait(?Send)]
impl ExecutorStorage for Storage {
    async fn get_flow(&self, id: i32) -> StorageResult<Option<Arc<Flow>>> {
        self.flows.get(id).await
    }
    async fn get_stage(&self, id: i32) -> StorageResult<Option<Arc<Stage>>> {
        self.stages.get(id).await
    }
    async fn get_policy(&self, id: i32) -> StorageResult<Option<Arc<Policy>>> {
        self.policies.get(id).await
    }
    async fn get_prompt(&self, id: i32) -> StorageResult<Option<Arc<Prompt>>> {
        self.prompts.get(id).await
    }
    async fn get_tenant(&self, id: i32) -> StorageResult<Option<Arc<Tenant>>> {
        self.tenants.get(id).await
    }
}

pub type MokaCache<T> = Cache<i32, Arc<Mutex<Option<Option<Arc<T>>>>>>;

struct DataCache<T, E> {
    getter: Box<dyn Fn(Object, i32) -> LocalBoxFuture<'static, Result<Option<T>, E>> + Send>,
    cache: MokaCache<T>,
    // locks: DashMap<i32, Mutex<Option<Option<Arc<T>>>>>,
    _error: PhantomData<E>,
    pool: Pool,
}

impl<T: Send + Sync + 'static, E: From<PoolError>> DataCache<T, E> {
    pub fn new<Fut: Future<Output = Result<Option<T>, E>> + 'static + Send>(
        cache: MokaCache<T>,
        pool: Pool,
        func: impl Fn(Object, i32) -> Fut + 'static + Send,
    ) -> Self {
        let func = Box::new(move |obj, id| func(obj, id).boxed_local());
        Self {
            getter: func,
            cache,
            _error: PhantomData,
            pool,
        }
    }

    pub async fn get(&self, id: i32) -> Result<Option<Arc<T>>, E> {
        let mutex = self
            .cache
            .get_with(id, async { Arc::new(Mutex::new(None)) })
            .await;
        let mut guard = mutex.lock().await;
        if let Some(value) = &*guard {
            return Ok(value.clone());
        }

        let conn = match self.pool.get().await {
            Ok(conn) => conn,
            Err(err) => {
                *guard = Some(None);
                return Err(err.into());
            }
        };

        let result = (self.getter)(conn, id).await.map(|opt| opt.map(Arc::new));
        match &result {
            Ok(v) => {
                *guard = Some(v.clone());
            }
            Err(_) => {
                *guard = Some(None);
            }
        }
        result
    }

    pub async fn invalidate(&self, id: i32) {
        self.cache.invalidate(&id).await;
    }
}

pub struct Storage {
    flows: DataCache<Flow, StorageError>,
    stages: DataCache<Stage, StorageError>,
    policies: DataCache<Policy, StorageError>,
    prompts: DataCache<Prompt, StorageError>,
    tenants: DataCache<Tenant, StorageError>,
}

impl Storage {
    pub fn new(pool: Pool) -> Self {
        Self {
            flows: DataCache::new(Cache::builder().build(), pool.clone(), flow_from_db),
            stages: DataCache::new(Cache::builder().build(), pool.clone(), stage_from_db),
            policies: DataCache::new(Cache::builder().build(), pool.clone(), policy_from_db),
            prompts: DataCache::new(Cache::builder().build(), pool.clone(), prompt_from_db),
            tenants: DataCache::new(Cache::builder().build(), pool.clone(), tenant_from_db),
        }
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

    pub async fn conflicts_for_flow(&self, client: &impl GenericClient, id: i32) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_stage(&self, client: &impl GenericClient, id: i32) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_policy(
        &self,
        client: &impl GenericClient,
        id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_prompt(
        &self,
        client: &impl GenericClient,
        id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }
    pub async fn conflicts_for_tenant(
        &self,
        client: &impl GenericClient,
        id: i32,
    ) -> Vec<Conflict> {
        vec![]
    }

    pub async fn get_flow(&self, id: i32) -> StorageResult<Option<Arc<Flow>>> {
        self.flows.get(id).await
    }
    pub async fn get_stage(&self, id: i32) -> StorageResult<Option<Arc<Stage>>> {
        self.stages.get(id).await
    }
    pub async fn get_policy(&self, id: i32) -> StorageResult<Option<Arc<Policy>>> {
        self.policies.get(id).await
    }
    pub async fn get_prompt(&self, id: i32) -> StorageResult<Option<Arc<Prompt>>> {
        self.prompts.get(id).await
    }
    pub async fn get_tenant(&self, id: i32) -> StorageResult<Option<Arc<Tenant>>> {
        self.tenants.get(id).await
    }
}

async fn flow_from_db(client: Object, id: i32) -> StorageResult<Option<Flow>> {
    let statement = client.prepare_cached(include_sql!("flow/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::flow::from_row(&client, row).await?)),
        None => Ok(None),
    }
}
async fn stage_from_db(client: Object, id: i32) -> StorageResult<Option<Stage>> {
    let statement = client.prepare_cached(include_sql!("stage/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::stage::from_row(&client, row).await?)),
        None => Ok(None),
    }
}
async fn policy_from_db(client: Object, id: i32) -> StorageResult<Option<Policy>> {
    let statement = client.prepare_cached(include_sql!("policy/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::policy::from_row(&client, row).await?)),
        None => Ok(None),
    }
}
async fn prompt_from_db(client: Object, id: i32) -> StorageResult<Option<Prompt>> {
    let statement = client.prepare_cached(include_sql!("prompt/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::prompt::from_row(row))),
        None => Ok(None),
    }
}
async fn tenant_from_db(client: Object, id: i32) -> StorageResult<Option<Tenant>> {
    let statement = client.prepare_cached(include_sql!("tenant/by-id")).await?;
    match client.query_opt(&statement, &[&id]).await? {
        Some(row) => Ok(Some(crate::tenant::from_row(row))),
        None => Ok(None),
    }
}
