use std::sync::Arc;

use deadpool_postgres::{Object, Pool};

use crate::auth::AuthState;

#[derive(Clone)]
pub struct AppState(Arc<InternalState>);

pub(super) struct InternalState {
    pool: Pool,
    auth: AuthState,
}

impl AppState {
    pub fn new(pool: Pool, auth: AuthState) -> Self {
        Self(Arc::new(InternalState { pool, auth }))
    }

    pub async fn conn(&self) -> Result<Object, deadpool_postgres::PoolError> {
        self.0.pool.get().await
    }

    pub fn auth(&self) -> &AuthState {
        &self.0.auth
    }
}
