use deadpool_postgres::GenericClient;
use model::user::PartialUser;
use sqlx::{query, Postgres};
use uuid::Uuid;

use crate::api::{sql_tx::Tx, ApiError};

#[derive(Clone)]
pub struct UserService {}

impl UserService {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserService {
    pub fn delete_user(&self, _uid: i32) {}

    pub async fn lookup_user_uid(
        &self,
        client: &impl GenericClient,
        uid: Uuid,
    ) -> Result<Option<PartialUser>, ApiError> {
        let statement = client
            .prepare_cached("select uid,name,administrator from users where uid = $1")
            .await?;
        let result = client.query_opt(&statement, &[&uid]).await?;
        Ok(if let Some(res) = result {
            Some(PartialUser {
                uid: res.get("uid"),
                name: res.get("name"),
                avatar_url: None,
                is_admin: res.get("administrator"),
            })
        } else {
            None
        })
    }

    pub async fn lookup_user(
        &self,
        tx: &mut Tx<Postgres>,
        text: &str,
        use_name: bool,
        use_email: bool,
        use_uuid: bool,
    ) -> Result<Option<PartialUser>, ApiError> {
        if use_name {
            if let Some(res) = query!(
                "select uid,name,administrator from users where name = $1",
                text
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                return Ok(Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    avatar_url: None,
                    is_admin: res.administrator,
                }));
            }
        }
        if use_email {
            if let Some(res) = query!(
                "select uid,name,administrator from users where email = $1",
                text
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                return Ok(Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    avatar_url: None,
                    is_admin: res.administrator,
                }));
            }
        }
        if use_uuid {
            let uuid = match Uuid::parse_str(text) {
                Ok(v) => v,
                Err(_) => return Ok(None),
            };
            if let Some(res) = query!(
                "select uid,name,administrator from users where uid = $1",
                uuid
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                return Ok(Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    avatar_url: None,
                    is_admin: res.administrator,
                }));
            }
        }
        return Ok(None);
    }
}
