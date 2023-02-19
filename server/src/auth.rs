use deadpool_postgres::GenericClient;
use model::user::PartialUser;
use uuid::Uuid;

use crate::{api::ApiError, SharedState};

pub struct Session {
    pub session_id: String,
    pub user_id: Option<Uuid>,
    pub is_admin: bool,
}

impl Session {
    pub async fn get_user(
        &self,
        connection: &impl GenericClient,
        state: &SharedState,
    ) -> Result<Option<PartialUser>, ApiError> {
        Ok(match self.user_id {
            Some(id) => state.users().lookup_user_uid(connection, id).await?,
            None => None,
        })
    }
}
