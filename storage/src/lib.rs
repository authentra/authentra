use async_trait::async_trait;
use std::{
    error::Error,
    fmt::{Debug, Display},
};

pub mod flow;
pub mod policy;
pub mod prompt;
pub mod stage;
mod storage;
pub mod tenant;

pub use storage::Conflict;
pub use storage::EntityId;
pub use storage::ExecutorStorage;
pub use storage::Storage;
pub use storage::StorageResult;

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
pub trait ReverseLookup<T> {
    async fn reverse_lookup(&self, sub: &T);
}
