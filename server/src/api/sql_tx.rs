use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    response::{IntoResponse, Response},
};
use derive_more::{Display, Error, From};
use futures::future::BoxFuture;
use http::{request::Parts, Request, StatusCode};
use parking_lot::Mutex;
use sqlx::{Database, Executor, Pool, Postgres, Transaction};
use tower::{Layer, Service};

#[derive(Debug)]
pub struct Tx<Db: Database>(Lease<sqlx::Transaction<'static, Db>>);

impl<Db: Database> Tx<Db> {
    pub async fn commit(mut self) -> Result<(), sqlx::Error> {
        self.0.steal().commit().await
    }
}

#[derive(Debug)]
pub struct TxLayer<Db: Database> {
    pool: Pool<Db>,
}

impl<Db: Database> Clone for TxLayer<Db> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

impl<Db: Database> TxLayer<Db> {
    pub fn new(pool: Pool<Db>) -> Self {
        Self { pool }
    }
}

impl<S, Db: Database> Layer<S> for TxLayer<Db> {
    type Service = TxService<S, Db>;

    fn layer(&self, inner: S) -> Self::Service {
        TxService {
            pool: self.pool.clone(),
            inner,
        }
    }
}

pub struct TxService<S, Db: Database> {
    pool: Pool<Db>,
    inner: S,
}

impl<S: Clone, Db: Database> Clone for TxService<S, Db> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<S, ReqBody: Send + 'static, Db: Database> Service<Request<ReqBody>> for TxService<S, Db>
where
    S: Service<Request<ReqBody>> + Send + Clone + 'static,
    S::Response: IntoResponse,
    S::Future: Send + 'static,
{
    type Response = Response;

    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let (slot, lease) = Slot::new_leased(None);
            let lazy = LazyTx { pool, tx: lease };
            req.extensions_mut().insert(lazy);
            let res = inner.call(req).await.map(IntoResponse::into_response)?;

            if res.status().is_success() {
                if let Some(tx) = slot.into_inner().flatten().and_then(Slot::into_inner) {
                    if let Err(err) = tx.commit().await {
                        tracing::error!("{err}");
                        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    }
                }
            }
            Ok(res)
        })
    }
}

#[async_trait]
impl<Db: Database, S> FromRequestParts<S> for Tx<Db> {
    type Rejection = TxError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ext: &mut LazyTx<Db> = parts
            .extensions
            .get_mut()
            .ok_or(TxError::MissingExtension)?;
        Ok(Self(ext.get_or_begin().await?))
    }
}

impl<Db: Database> AsRef<Transaction<'static, Db>> for Tx<Db> {
    fn as_ref(&self) -> &Transaction<'static, Db> {
        &self.0
    }
}
impl<Db: Database> AsMut<Transaction<'static, Db>> for Tx<Db> {
    fn as_mut(&mut self) -> &mut Transaction<'static, Db> {
        &mut self.0
    }
}

impl<'c> Executor<'c> for &'c mut Tx<Postgres> {
    type Database = Postgres;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> futures_util::stream::BoxStream<
        'e,
        Result<
            sqlx::Either<
                <Self::Database as Database>::QueryResult,
                <Self::Database as Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        self.0.deref_mut().fetch_many(query)
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> futures_util::future::BoxFuture<
        'e,
        Result<Option<<Self::Database as Database>::Row>, sqlx::Error>,
    >
    where
        'c: 'e,
        E: sqlx::Execute<'q, Self::Database>,
    {
        self.0.deref_mut().fetch_optional(query)
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as Database>::TypeInfo],
    ) -> futures_util::future::BoxFuture<
        'e,
        Result<<Self::Database as sqlx::database::HasStatement<'q>>::Statement, sqlx::Error>,
    >
    where
        'c: 'e,
    {
        self.0.deref_mut().prepare_with(sql, parameters)
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> futures_util::future::BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        self.0.deref_mut().describe(sql)
    }
}

pub struct LazyTx<Db: Database> {
    pool: Pool<Db>,
    tx: Lease<Option<Slot<Transaction<'static, Db>>>>,
}

#[derive(Debug, Display, Error, From)]
pub enum TxError {
    #[display("Overlapping extractors")]
    OverlappingExtractors,

    #[display("Missing sql tx extension")]
    MissingExtension,

    #[from(forward)]
    Database(#[error(source)] sqlx::Error),
}

impl IntoResponse for TxError {
    fn into_response(self) -> Response {
        tracing::error!("{self}");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl<Db: Database> LazyTx<Db> {
    async fn get_or_begin(&mut self) -> Result<Lease<Transaction<'static, Db>>, TxError> {
        let tx = if let Some(tx) = self.tx.as_mut() {
            tx
        } else {
            let tx = self.pool.begin().await?;
            self.tx.insert(Slot::new(tx))
        };
        tx.lease().ok_or(TxError::OverlappingExtractors)
    }
}

struct Slot<T>(Arc<Mutex<Option<T>>>);

impl<T> Slot<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(Some(value))))
    }
    fn new_leased(value: T) -> (Self, Lease<T>) {
        let slot = Self::new(value);
        let lease = slot.lease().expect("New slot is empty");
        (slot, lease)
    }

    fn lease(&self) -> Option<Lease<T>> {
        if let Some(value) = self.0.try_lock().and_then(|mut slot| slot.take()) {
            Some(Lease::new(value, Arc::downgrade(&self.0)))
        } else {
            None
        }
    }

    fn into_inner(self) -> Option<T> {
        self.0.try_lock().and_then(|mut slot| slot.take())
    }
}

#[derive(Debug)]
struct Lease<T>(Inner<T>);

impl<T> Drop for Lease<T> {
    fn drop(&mut self) {
        self.drop_self();
    }
}

impl<T> Deref for Lease<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Lease<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> Lease<T> {
    pub fn new(value: T, slot: Weak<Mutex<Option<T>>>) -> Self {
        Self(Inner::Value { value, slot })
    }

    fn as_ref(&self) -> &T {
        match &self.0 {
            Inner::Value { value, .. } => &value,
            Inner::Stolen | Inner::Dropped => panic!("BUG: Lease used after drop/steal"),
        }
    }
    fn as_mut(&mut self) -> &mut T {
        match &mut self.0 {
            Inner::Value { value, .. } => value,
            Inner::Stolen | Inner::Dropped => panic!("BUG: Lease used after drop/steal"),
        }
    }
    fn drop_self(&mut self) {
        if let Inner::Value { value, slot } = std::mem::replace(&mut self.0, Inner::Dropped) {
            if let Some(slot) = slot.upgrade() {
                if let Some(mut lock) = slot.try_lock() {
                    *lock = Some(value);
                }
            }
        }
    }

    pub fn steal(&mut self) -> T {
        match std::mem::replace(&mut self.0, Inner::Stolen) {
            Inner::Value { value, .. } => value,
            Inner::Stolen => panic!("BUG: Lease steal called twice"),
            Inner::Dropped => panic!("BUG: Lease used after drop"),
        }
    }
}

#[derive(Debug)]
enum Inner<T> {
    Value {
        value: T,
        slot: Weak<Mutex<Option<T>>>,
    },
    Stolen,
    Dropped,
}
