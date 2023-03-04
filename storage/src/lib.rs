use std::{error::Error, fmt::Display};

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

datacache::storage_ref!(pub StorageRef);
// ($vis:vis $ident:ident($exc:ty, $data:ty), id($id_field:ident: $id_ty:ty), unique($($unique:ident: $unique_ty:ty),* ), fields($($field:ident: $field_ty:ty),* )) => {

datacache::storage_manager!(pub StorageManager: StorageRef);

#[macro_export]
macro_rules! include_sql {
    ($path:literal) => {
        include_str!(concat!("sql/", $path, ".sql"))
    };
}

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
