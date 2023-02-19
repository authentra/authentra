use deadpool_postgres::GenericClient;
use model::user::PartialUser;
use uuid::Uuid;

use crate::api::ApiError;

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
            .prepare_cached(
                "select uid,name,administrator,password_change_date from users where uid = $1",
            )
            .await?;
        let result = client.query_opt(&statement, &[&uid]).await?;
        Ok(if let Some(res) = result {
            Some(PartialUser {
                uid: res.get("uid"),
                name: res.get("name"),
                avatar_url: None,
                is_admin: res.get("administrator"),
                password_change_date: res.get("password_change_date"),
            })
        } else {
            None
        })
    }

    pub async fn lookup_user(
        &self,
        client: &impl GenericClient,
        text: &str,
        use_name: bool,
        use_email: bool,
        use_uuid: bool,
    ) -> Result<Option<PartialUser>, ApiError> {
        if use_name {
            let statement = client
                .prepare_cached(
                    "select uid,name,administrator,password_change_date from users where name = $1",
                )
                .await?;
            let result = client.query_opt(&statement, &[&text]).await?;
            if let Some(res) = result {
                return Ok(Some(PartialUser {
                    uid: res.get("uid"),
                    name: res.get("name"),
                    avatar_url: None,
                    is_admin: res.get("administrator"),
                    password_change_date: res.get("password_change_date"),
                }));
            }
        }
        if use_email {
            let statement = client
                .prepare_cached(
                    "select uid,name,administrator,password_change_date from users where email = $1",
                )
                .await?;
            let result = client.query_opt(&statement, &[&text]).await?;
            if let Some(res) = result {
                return Ok(Some(PartialUser {
                    uid: res.get("uid"),
                    name: res.get("name"),
                    avatar_url: None,
                    is_admin: res.get("administrator"),
                    password_change_date: res.get("password_change_date"),
                }));
            }
        }
        if use_uuid {
            let uuid = match Uuid::parse_str(text) {
                Ok(v) => v,
                Err(_) => return Ok(None),
            };
            let statement = client
                .prepare_cached(
                    "select uid,name,administrator,password_change_date from users where uid = $1",
                )
                .await?;
            let result = client.query_opt(&statement, &[&uuid]).await?;
            if let Some(res) = result {
                return Ok(Some(PartialUser {
                    uid: res.get("uid"),
                    name: res.get("name"),
                    avatar_url: None,
                    is_admin: res.get("administrator"),
                    password_change_date: res.get("password_change_date"),
                }));
            }
        }
        return Ok(None);
    }
}
