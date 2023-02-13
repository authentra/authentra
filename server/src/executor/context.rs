use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

use model::{PendingUser, Reference, Stage};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::flow_storage::FreezedStorage;

pub struct ExecutionContext {
    pub session_id: String,
    pub start_time: OffsetDateTime,
    pub fields: FieldStorage,
    pub pending: Option<PendingUser>,
    pub user: Option<PolicyUser>,
    pub storage: FreezedStorage,
    pub error: Option<ExecutionError>,
}

pub struct ExecutionError {
    pub stage: Option<Reference<Stage>>,
    pub message: String,
}

impl ExecutionContext {
    pub fn new(session_id: String, storage: FreezedStorage) -> Self {
        Self {
            session_id,
            start_time: OffsetDateTime::now_utc(),
            fields: FieldStorage::new(),
            user: None,
            storage,
            pending: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyUser {
    pub uid: Uuid,
    pub name: String,
    pub password_change_date: OffsetDateTime,
}

pub enum FieldStorageError {
    WrongType,
}

pub struct FieldStorage {
    fields: HashMap<String, Box<dyn Any + Send + Sync>>,
}

pub struct FieldKey<Value: Any> {
    pub name: &'static str,
    _value: PhantomData<Value>,
}

impl<Value: Any> FieldKey<Value> {
    pub const fn new(key: &'static str) -> Self {
        Self {
            name: key,
            _value: PhantomData,
        }
    }
}

impl FieldStorage {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    #[inline(always)]
    pub fn get_typed<T: Any>(&self, key: FieldKey<T>) -> Result<Option<&T>, FieldStorageError> {
        self.get_dynamic(key.name)
    }
    pub fn get_dynamic<T: Any>(&self, field_name: &str) -> Result<Option<&T>, FieldStorageError> {
        if let Some(entry) = self.fields.get(field_name) {
            let id = TypeId::of::<T>();
            if entry.type_id() != id {
                Err(FieldStorageError::WrongType)
            } else {
                Ok(Some(entry.downcast_ref().expect("Downcast failed")))
            }
        } else {
            Ok(None)
        }
    }

    #[inline(always)]
    pub fn insert_typed<T: Any + Send + Sync + 'static>(&mut self, key: FieldKey<T>, value: T) {
        self.insert_dynamic(key.name, value)
    }

    pub fn insert_dynamic<T: Any + Send + Sync + 'static>(&mut self, name: &str, value: T) {
        self.fields.insert(name.to_owned(), Box::new(value));
    }
}
