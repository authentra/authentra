use async_trait::async_trait;
use datacache::{Data, DataMarker, DataQueryExecutor, DataStorage};
use deadpool_postgres::Pool;
use flow::{FlowExecutor, FlowStorage};
use parking_lot::Mutex;
use policy::{PolicyExecutor, PolicyStorage};
use prompt::{PromptExecutor, PromptStorage};
use stage::{StageExecutor, StageStorage};
use std::{
    collections::HashMap,
    convert::Infallible,
    error::Error,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
};
use tenant::{TenantExecutor, TenantStorage};

pub mod flow;
pub mod policy;
pub mod prompt;
pub mod stage;
pub mod tenant;

pub use datacache;

macro_rules! executor {
    ($vis:vis $ident:ident) => {
        $vis struct $ident {
            pool: deadpool_postgres::Pool,
        }

        impl $ident {
            pub fn new(pool: deadpool_postgres::Pool) -> Self {
                Self {
                    pool,
                }
            }

            #[inline(always)]
            async fn get_conn(&self) -> Result<deadpool_postgres::Object, deadpool_postgres::PoolError> {
                self.pool.get().await
            }
        }
    };
}

pub(crate) use executor;

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

datacache::storage_ref!(pub StorageRef);
// datacache::storage_ref!(pub FreezedRef);
// ($data:path: $ref:ident where Exc: $exc:path, Storage: $storage:path) => {

datacache::storage_manager!(pub StorageManager: StorageRef);
datacache::storage_lookup!(StorageManager: StorageRef, handle_error);

datacache::storage_ref!(model::Flow: StorageRef where Exc: flow::FlowExecutor, Storage: flow::FlowStorage);
datacache::storage_ref!(model::Stage: StorageRef where Exc: stage::StageExecutor, Storage: stage::StageStorage);
datacache::storage_ref!(model::Policy: StorageRef where Exc: policy::PolicyExecutor, Storage: policy::PolicyStorage);
datacache::storage_ref!(model::Prompt: StorageRef where Exc: prompt::PromptExecutor, Storage: prompt::PromptStorage);
datacache::storage_ref!(model::Tenant: StorageRef where Exc: tenant::TenantExecutor, Storage: tenant::TenantStorage);

// datacache::storage_manager!(pub FreezedManager: FreezedRef, handle_error);

// datacache::storage_ref!(model::Flow: FreezedRef where Exc: DummyExecutor<i32>, Storage: DummyStorage<model::Flow, i32>);
// datacache::storage_ref!(model::Stage: FreezedRef where Exc: DummyExecutor<i32>, Storage: DummyStorage<model::Stage, i32>);
// datacache::storage_ref!(model::Policy: FreezedRef where Exc: DummyExecutor<i32>, Storage: DummyStorage<model::Policy, i32>);
// datacache::storage_ref!(model::Prompt: FreezedRef where Exc: DummyExecutor<i32>, Storage: DummyStorage<model::Prompt, i32>);
// datacache::storage_ref!(model::Tenant: FreezedRef where Exc: DummyExecutor<i32>, Storage: DummyStorage<model::Tenant, i32>);

datacache::storage_lookup!(FreezedStorage: StorageRef, handle_error, get_data_freezed);

pub struct FreezedStorage(StorageManager);

fn get_data_freezed<D: DataMarker + 'static>(
    storage: &FreezedStorage,
) -> Option<&DummyStorage<D, i32>> {
    storage.0.get_storage()
}
pub struct ProxiedStorage(StorageManager);

fn get_data_proxied<D: DataMarker + StorageRef + 'static>(
    storage: &ProxiedStorage,
) -> Option<&ProxyStorage<D::Storage, D::Exc, D>> {
    storage.0.get_storage()
}
datacache::storage_lookup!(ProxiedStorage: StorageRef, handle_error, get_data_proxied);

fn handle_error(err: impl Display) {
    tracing::error!("An error occurred {err}");
}

pub fn create_manager(pool: Pool) -> StorageManager {
    let mut manager = StorageManager::new();
    manager.register_storage(FlowStorage::new(FlowExecutor::new(pool.clone())));
    manager.register_storage(StageStorage::new(StageExecutor::new(pool.clone())));
    manager.register_storage(PolicyStorage::new(PolicyExecutor::new(pool.clone())));
    manager.register_storage(PromptStorage::new(PromptExecutor::new(pool.clone())));
    manager.register_storage(TenantStorage::new(TenantExecutor::new(pool)));
    manager
}

pub fn create_proxied(manager: &StorageManager) -> ProxiedStorage {
    let mut proxied = StorageManager::new();
    register_proxied::<model::Flow>(&manager, &mut proxied);
    register_proxied::<model::Stage>(&manager, &mut proxied);
    register_proxied::<model::Policy>(&manager, &mut proxied);
    register_proxied::<model::Prompt>(&manager, &mut proxied);
    register_proxied::<model::Tenant>(&manager, &mut proxied);
    ProxiedStorage(proxied)
}

pub fn create_freezed(mut manager: ProxiedStorage) -> FreezedStorage {
    let flow = get_proxied::<model::Flow>(&mut manager).export_data();
    let stage = get_proxied::<model::Stage>(&mut manager).export_data();
    let policy = get_proxied::<model::Policy>(&mut manager).export_data();
    let prompt = get_proxied::<model::Prompt>(&mut manager).export_data();
    let tenant = get_proxied::<model::Tenant>(&mut manager).export_data();
    let mut manager = StorageManager::new();
    manager.register_storage(DummyStorage::new(flow));
    manager.register_storage(DummyStorage::new(stage));
    manager.register_storage(DummyStorage::new(policy));
    manager.register_storage(DummyStorage::new(prompt));
    manager.register_storage(DummyStorage::new(tenant));
    FreezedStorage(manager)
}

fn register_proxied<D: StorageRef + Send + Sync + 'static>(
    proxied: &StorageManager,
    manager: &mut StorageManager,
) {
    let storage = proxied.get_for_data::<D>().expect("No storage for data");
    manager.register_storage(ProxyStorage::new(storage.clone()));
}

fn get_proxied<D: StorageRef + Send + Sync + 'static>(
    manager: &mut ProxiedStorage,
) -> Arc<ProxyStorage<<D as StorageRef>::Storage, <D as StorageRef>::Exc, D>> {
    manager
        .0
        .get_and_remove::<ProxyStorage<D::Storage, D::Exc, D>>()
        .expect("Failed to find proxied")
}

#[macro_export]
macro_rules! include_sql {
    ($path:literal) => {
        include_str!(concat!("sql/", $path, ".sql"))
    };
}

struct ProxyStorage<P: DataStorage<Exc, D>, Exc: DataQueryExecutor<D>, D: DataMarker> {
    proxy: P,
    _exc: PhantomData<Exc>,
    _d: PhantomData<D>,
    data: Mutex<Option<HashMap<Exc::Id, Data<D>>>>,
    query: Mutex<Option<HashMap<D::Query, Exc::Id>>>,
}

pub struct ExportedData<Query, Id, D> {
    data: HashMap<Id, Data<D>>,
    query: HashMap<Query, Id>,
}

impl<P, Exc, D> ProxyStorage<P, Exc, D>
where
    P: DataStorage<Exc, D>,
    Exc: DataQueryExecutor<D>,
    Exc::Id: Hash + Eq + Clone,
    D: DataMarker,
    D::Query: Hash + Eq,
{
    pub fn new(proxy: P) -> Self {
        Self {
            proxy,
            _exc: PhantomData,
            _d: PhantomData,
            data: Mutex::new(Some(HashMap::new())),
            query: Mutex::new(Some(HashMap::new())),
        }
    }

    fn insert_data(&self, data: Data<D>) {
        let id = self.proxy.get_executor().get_id(data.as_ref());
        let queries = data.create_queries();
        let mut raw_lock = self.query.lock();
        let lock = raw_lock.as_mut().expect("Already exported");
        for query in queries {
            lock.insert(query, id.clone());
        }
        self.data
            .lock()
            .as_mut()
            .expect("Already exported")
            .insert(id, data);
    }

    pub fn export_data(&self) -> ExportedData<D::Query, Exc::Id, D> {
        let data = self.data.lock().take().expect("Mutex doesn't contain data");
        let query = self
            .query
            .lock()
            .take()
            .expect("Mutex doesn't contain queries");
        ExportedData { data, query }
    }
}

#[async_trait]
impl<P, Exc, D> DataStorage<Exc, D> for ProxyStorage<P, Exc, D>
where
    P: DataStorage<Exc, D> + Send + Sync,
    Exc: DataQueryExecutor<D> + Send + Sync,
    Exc::Id: Hash + Eq + Clone,
    D: DataMarker + Send + Sync,
    D::Query: Hash + Eq,
{
    async fn find_one(&self, query: &D::Query) -> Result<Data<D>, Exc::Error> {
        self.proxy.find_one(query).await.map(|v| {
            self.insert_data(v.clone());
            v
        })
    }
    async fn find_all(&self, query: Option<&D::Query>) -> Result<Vec<Data<D>>, Exc::Error> {
        self.proxy.find_all(query).await
    }
    async fn find_optional(&self, query: &D::Query) -> Result<Option<Data<D>>, Exc::Error> {
        self.proxy.find_optional(query).await.map(|opt| {
            opt.map(|v| {
                self.insert_data(v.clone());
                v
            })
        })
    }

    async fn delete(&self, _query: &D::Query) -> Result<(), Exc::Error> {
        panic!("Delete is unsupported on a proxy datastorage");
    }
    async fn invalidate(&self, _query: &D::Query) -> Result<(), Exc::Error> {
        panic!("Invalidate is unsupported on a proxy datastorage");
    }

    fn get_executor(&self) -> &Exc {
        self.proxy.get_executor()
    }
}

pub struct DummyExecutor<Id> {
    _id: PhantomData<Id>,
}

impl<D: DataMarker, Id: Send + Sync + Hash + Eq + Clone> DataQueryExecutor<D>
    for DummyExecutor<Id>
{
    type Error = Infallible;

    type Id = Id;

    fn get_id(&self, _data: &D) -> Self::Id {
        panic!("Unsupported operation")
    }

    fn find_one<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _query: &'life1 <D as DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<D, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        panic!("Unsupported operation")
    }

    fn find_all_ids<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _query: Option<&'life1 <D as DataMarker>::Query>,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Vec<Self::Id>, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        panic!("Unsupported operation")
    }

    fn find_optional<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _query: &'life1 <D as DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Option<D>, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        panic!("Unsupported operation")
    }

    fn delete<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _data: &'life1 <D as DataMarker>::Query,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Vec<Self::Id>, Self::Error>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        panic!("Unsupported operation")
    }
}

#[derive(Clone)]
pub struct DummyStorage<D: DataMarker, Id> {
    data: Arc<ExportedData<D::Query, Id, D>>,
}

impl<D, Id> DummyStorage<D, Id>
where
    D: DataMarker,
    D::Query: Hash + Eq + Debug,
    Id: Hash + Eq + Debug,
{
    pub fn new(data: ExportedData<D::Query, Id, D>) -> Self {
        Self {
            data: Arc::new(data),
        }
    }

    fn find_id(&self, query: &D::Query) -> &Id {
        match self.data.query.get(query) {
            Some(id) => id,
            None => panic!("Failed to find data for {query:?}"),
        }
    }

    fn get_data(&self, id: &Id) -> Result<Data<D>, Infallible> {
        Ok(match self.data.data.get(id) {
            Some(data) => data.clone(),
            None => panic!("Failed to find data for {id:?}"),
        })
    }
}

#[async_trait]
impl<D, Id> DataStorage<DummyExecutor<Id>, D> for DummyStorage<D, Id>
where
    D: DataMarker + Send + Sync,
    D::Query: Hash + Eq + Debug,
    Id: Send + Sync + Hash + Eq + Debug + Clone,
{
    async fn find_one(&self, query: &D::Query) -> Result<Data<D>, Infallible> {
        let id = self.find_id(&query);
        self.get_data(id)
    }
    async fn find_all(&self, _: Option<&D::Query>) -> Result<Vec<Data<D>>, Infallible> {
        panic!("Unsupported operation")
    }
    async fn find_optional(&self, query: &D::Query) -> Result<Option<Data<D>>, Infallible> {
        let id = self.find_id(&query);
        Ok(self.data.data.get(id).cloned())
    }

    async fn delete(&self, _: &D::Query) -> Result<(), Infallible> {
        panic!("Unsupported operation")
    }
    async fn invalidate(&self, _: &D::Query) -> Result<(), Infallible> {
        panic!("Unsupported operation")
    }

    fn get_executor(&self) -> &DummyExecutor<Id> {
        panic!("Unsupported operation")
    }
}

#[async_trait]
pub trait ReverseLookup<T> {
    async fn reverse_lookup(&self, sub: &T);
}
