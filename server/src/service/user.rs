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
        tx: &mut Tx<Postgres>,
        uid: Uuid,
    ) -> Result<Option<PartialUser>, ApiError> {
        Ok(
            if let Some(res) = query!("select uid,name from users where uid = $1", uid)
                .fetch_optional(&mut *tx)
                .await?
            {
                Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    icon_url: None,
                })
            } else {
                None
            },
        )
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
            if let Some(res) = query!("select uid,name from users where name = $1", text)
                .fetch_optional(&mut *tx)
                .await?
            {
                return Ok(Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    icon_url: None,
                }));
            }
        }
        if use_email {
            if let Some(res) = query!("select uid,name from users where email = $1", text)
                .fetch_optional(&mut *tx)
                .await?
            {
                return Ok(Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    icon_url: None,
                }));
            }
        }
        if use_uuid {
            let uuid = match Uuid::parse_str(text) {
                Ok(v) => v,
                Err(_) => return Ok(None),
            };
            if let Some(res) = query!("select uid,name from users where uid = $1", uuid)
                .fetch_optional(&mut *tx)
                .await?
            {
                return Ok(Some(PartialUser {
                    uid: res.uid,
                    name: res.name,
                    icon_url: None,
                }));
            }
        }
        return Ok(None);
    }
}
