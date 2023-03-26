use std::{hash::Hash, marker::PhantomData, sync::Arc};

use deadpool_postgres::{GenericClient, Object, Pool, PoolError};
use futures::{future::BoxFuture, Future, FutureExt};
use model::{Flow, Policy, Prompt, Stage, Tenant};
use moka::future::Cache;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{include_sql, StorageError};

pub type MokaCache<I, T> = Cache<I, Arc<Mutex<Option<Option<Arc<T>>>>>>;
pub type GetterFunc<I, T, E> =
    dyn Fn(Object, I) -> BoxFuture<'static, Result<Option<T>, E>> + Send + Sync;
struct DataCache<I, T, E> {
    getter: Arc<GetterFunc<I, T, E>>,
    cache: MokaCache<I, T>,
    // locks: DashMap<i32, Mutex<Option<Option<Arc<T>>>>>,
    _error: PhantomData<E>,
    pool: Pool,
}

impl<I, T, E> Clone for DataCache<I, T, E> {
    fn clone(&self) -> Self {
        Self {
            getter: self.getter.clone(),
            cache: self.cache.clone(),
            _error: PhantomData,
            pool: self.pool.clone(),
        }
    }
}

#[allow(dead_code)]
impl<
        I: PartialEq + Eq + Hash + Send + Sync + Clone + 'static,
        T: Send + Sync + 'static,
        E: From<PoolError> + Send + Sync,
    > DataCache<I, T, E>
{
    pub fn new<Fut: Future<Output = Result<Option<T>, E>> + 'static + Send>(
        cache: MokaCache<I, T>,
        pool: Pool,
        func: impl Fn(Object, I) -> Fut + 'static + Send + Sync,
    ) -> Self {
        let func = Arc::new(move |obj, id| func(obj, id).boxed());
        Self {
            getter: func,
            cache,
            _error: PhantomData,
            pool,
        }
    }

    pub async fn get(&self, id: &I) -> Result<Option<Arc<T>>, E> {
        let mutex = self
            .cache
            .get_with_by_ref(id, async { Arc::new(Mutex::new(None)) })
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

        let result = (self.getter)(conn, id.clone())
            .await
            .map(|opt| opt.map(Arc::new));
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

    pub async fn invalidate(&self, id: &I) {
        self.cache.invalidate(id).await;
    }
}
