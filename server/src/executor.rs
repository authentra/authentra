use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use ::storage::ExecutorStorage;
use deadpool_postgres::{GenericClient, Object, Transaction};
use derive_more::Display;
use moka::sync::Cache;
use tokio::sync::{Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use tokio_postgres::{types::ToSql, Row, Statement, ToStatement};

use crate::{api::ApiError, auth::Session, service::policy::PolicyService};
use model::{PolicyKind, PolicyResult};

use self::flow::{FlowExecution, FlowExecutionInternal};

mod context;
pub mod data;
pub mod flow;
pub mod storage;
pub use context::*;

// 12 Hours
const TIME_TO_IDLE: Duration = Duration::from_secs(60 * 60 * 12);
// 36 Hours
const TIME_TO_LIVE: Duration = Duration::from_secs(60 * 60 * 36);

#[derive(Debug, Display, Clone, Hash, PartialEq, Eq)]
#[display("FlowKey({session}, {flow:?})")]
pub struct FlowKey {
    session: String,
    flow: i32,
}

impl FlowKey {
    fn new(session: &Session, flow: i32) -> Self {
        Self {
            session: session.session_id.clone(),
            flow,
        }
    }
}

#[derive(Clone)]
pub struct FlowExecutor {
    internal: Arc<FlowExecutorInternal>,
}

struct FlowExecutorInternal {
    executions: Cache<FlowKey, FlowExecution>,
    storage: Arc<dyn ExecutorStorage>,
    policy_service: PolicyService,
}

impl FlowExecutorInternal {
    pub fn new(storage: Arc<dyn ExecutorStorage>, policy_service: PolicyService) -> Self {
        Self {
            executions: Cache::builder()
                .time_to_idle(TIME_TO_IDLE)
                .time_to_live(TIME_TO_LIVE)
                .build(),
            storage,
            policy_service,
        }
    }
}

impl FlowExecutor {
    pub fn new(storage: Arc<dyn ExecutorStorage>, policy_service: PolicyService) -> Self {
        Self {
            internal: Arc::new(FlowExecutorInternal::new(storage, policy_service)),
        }
    }

    pub fn invalidate_flow(&self, key: &FlowKey) {
        self.internal.executions.invalidate(key);
    }

    pub fn get_key(&self, session: &Session, flow: i32) -> FlowKey {
        FlowKey::new(session, flow)
    }

    pub async fn get_execution(
        &self,
        key: &FlowKey,
        start: bool,
    ) -> Result<Option<FlowExecution>, ApiError> {
        let execution = self.internal.executions.get(key);
        if execution.is_some() {
            return Ok(execution);
        }

        if start {
            self.start(key).await
        } else {
            Ok(None)
        }
    }

    pub async fn start(&self, key: &FlowKey) -> Result<Option<FlowExecution>, ApiError> {
        let flow = match self.internal.storage.get_flow(key.flow).await? {
            Some(flow) => flow,
            None => {
                return Ok(None);
            }
        };
        // let proxied = ::storage::create_proxied(&self.internal.storage_old);
        // if flow.entries.is_empty() {
        //     return None;
        // }
        // proxied.reverse_lookup(&*flow).await;
        // let storage = ::storage::create_freezed(proxied);
        let storage = self.internal.storage.clone();
        let context = ExecutionContext::new(key.session.clone(), storage.clone());
        let execution = FlowExecutionInternal {
            flow,
            context: RwLock::new(context),
            storage,
            current_entry_idx: parking_lot::Mutex::new(0),
            is_completed: AtomicBool::new(false),
            executor: self.clone(),
            key: key.clone(),
            policy_service: self.internal.policy_service.clone(),
        };
        let execution = FlowExecution(Arc::new(execution));
        self.internal
            .executions
            .insert(key.clone(), execution.clone());
        Ok(Some(execution))
    }

    // pub fn init(&mut self) {}
}

pub trait Validate {
    type Res;

    fn validate(&self, context: &ExecutionContext) -> Self::Res;
}

impl Validate for PolicyKind {
    type Res = PolicyResult;
    fn validate(&self, context: &ExecutionContext) -> Self::Res {
        match self {
            PolicyKind::PasswordExpiry { max_age } => context
                .user
                .as_ref()
                .map(|user| user.password_change_date)
                .map_or(PolicyResult::NotApplicable, |date| {
                    ((context.start_time - date) > time::Duration::seconds(*max_age as i64)).into()
                }),
            PolicyKind::PasswordStrength => todo!(),
            PolicyKind::Expression(..) => todo!(),
        }
    }
}

// pub struct DbClient<'a>(DbClientInner<'a>);

// enum DbClientInner<'a> {
//     Object(deadpool_postgres::Object),
//     Transaction(Transaction<'a>),
// }

macro_rules! func {
    ($self:expr, $($x:tt)*) => {
        match $self {
            DbClientInner::Object(obj) => obj.$($x)*,
            DbClientInner::Transaction(obj) => obj.$($x)*,
        }
    };
}

pub struct DbClient<'a> {
    client: RwLock<Object>,
    tx: Option<TxWrapper<'a>>,
}

struct TxWrapper<'a> {
    guard: RwLockWriteGuard<'a, Object>,
    tx: Option<Transaction<'a>>,
}

// impl<'a> DbClient<'a> {
//     pub fn new_client(client: Object) -> Self {
//         Self(DbClientInner::Object(client))
//     }
//     pub fn new_transaction(tx: Transaction<'a>) -> Self {
//         Self(DbClientInner::Transaction(tx))
//     }

//     pub async fn prepare(&self, query: &str) -> Result<Statement, tokio_postgres::Error> {
//         func!(&self.0, prepare_cached(query).await)
//     }

//     pub async fn execute<T>(
//         &self,
//         query: &T,
//         params: &[&(dyn ToSql + Sync)],
//     ) -> Result<u64, tokio_postgres::Error>
//     where
//         T: ?Sized + ToStatement + Sync + Send,
//     {
//         func!(&self.0, execute(query, params).await)
//     }
//     pub async fn query<T>(
//         &self,
//         query: &T,
//         params: &[&(dyn ToSql + Sync)],
//     ) -> Result<Vec<Row>, tokio_postgres::Error>
//     where
//         T: ?Sized + ToStatement + Sync + Send,
//     {
//         func!(&self.0, query(query, params).await)
//     }
//     pub async fn query_one<T>(
//         &self,
//         query: &T,
//         params: &[&(dyn ToSql + Sync)],
//     ) -> Result<Row, tokio_postgres::Error>
//     where
//         T: ?Sized + ToStatement + Sync + Send,
//     {
//         func!(&self.0, query_one(query, params).await)
//     }
//     pub async fn query_opt<T>(
//         &self,
//         query: &T,
//         params: &[&(dyn ToSql + Sync)],
//     ) -> Result<Option<Row>, tokio_postgres::Error>
//     where
//         T: ?Sized + ToStatement + Sync + Send,
//     {
//         func!(&self.0, query_opt(query, params).await)
//     }

//     pub async fn transaction(&mut self) -> Result<Transaction, tokio_postgres::Error> {
//         func!(&mut self.0, transaction().await)
//     }

//     pub async fn make_transaction(&mut self) -> Result<(), tokio_postgres::Error> {
//         let tx = match &mut self.0 {
//             DbClientInner::Object(obj) => {
//                 obj.transaction().await?
//                 // std::mem::replace(&mut self.0, obj.transaction());
//             }
//             DbClientInner::Transaction(_) => todo!(),
//         };
//         self.0 = DbClientInner::Transaction(tx);

//         Ok(())
//     }
// }

impl<'a> DbClient<'a> {
    pub async fn make_transaction(&'a mut self) -> Result<(), tokio_postgres::Error> {
        if self.tx.is_some() {
            panic!("Already is an transaction");
        }
        let guard = self.client.write().await;
        let mut wrapper = TxWrapper { guard, tx: None };
        // wrapper.tx = Some(wrapper.guard.transaction().await?);
        // let tx = guard.transaction().await?;
        self.tx = Some(wrapper);
        Ok(())
    }
}

impl<'a> TxWrapper<'a> {
    async fn make_tx(&'a mut self) -> Result<(), tokio_postgres::Error> {
        let tx = self.guard.transaction().await?;
        self.tx = Some(tx);
        Ok(())
    }
}

pub fn test(client: Object) {
    // let client = DbClient::new_client(client);
}
