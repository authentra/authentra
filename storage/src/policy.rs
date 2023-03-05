use async_trait::async_trait;
use datacache::{Data, DataQueryExecutor};
use deadpool_postgres::GenericClient;
use model::{Policy, PolicyKind, PolicyKindSimple, PolicyQuery};
use tokio_postgres::Row;

use crate::{include_sql, StorageError};

datacache::storage!(pub PolicyStorage(PolicyExecutor, Policy), id(uid: i32), unique(slug: String), fields());

crate::executor!(pub PolicyExecutor);

#[async_trait]
impl DataQueryExecutor<Policy> for PolicyExecutor {
    type Error = StorageError;
    type Id = i32;

    fn get_id(&self, data: &Policy) -> Self::Id {
        data.uid
    }

    async fn find_one(&self, query: &PolicyQuery) -> Result<Policy, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            PolicyQuery::uid(uid) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("policy/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            PolicyQuery::slug(slug) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("policy/by-slug")).await?,
                    &[&slug],
                )
                .await?
            }
        };
        from_row(&conn, row).await
    }
    async fn find_all_ids(
        &self,
        query: Option<&PolicyQuery>,
    ) -> Result<Vec<Self::Id>, Self::Error> {
        if let Some(query) = query {
            match query {
                PolicyQuery::uid(id) => return Ok(vec![id.clone()]),
                PolicyQuery::slug(_slug) => todo!(),
            }
        } else {
            let conn = self.get_conn().await?;
            let statement = conn.prepare_cached(include_sql!("policy/all-ids")).await?;
            let ids = conn.query(&statement, &[]).await?;
            Ok(ids.into_iter().map(|row| row.get("uid")).collect())
        }
    }
    async fn find_optional(&self, query: &PolicyQuery) -> Result<Option<Policy>, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            PolicyQuery::uid(uid) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("policy/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            PolicyQuery::slug(slug) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("policy/by-slug")).await?,
                    &[&slug],
                )
                .await?
            }
        };
        Ok(match row {
            Some(row) => Some(from_row(&conn, row).await?),
            None => None,
        })
    }
    async fn create(&self, _data: Data<Policy>) -> Result<(), Self::Error> {
        todo!()
    }
    async fn update(&self, _data: Data<Policy>) -> Result<(), Self::Error> {
        todo!()
    }
    async fn delete(&self, _data: &PolicyQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
}

async fn from_row(client: &impl GenericClient, row: Row) -> Result<Policy, StorageError> {
    let simple_kind: PolicyKindSimple = row.get("kind");
    let kind = match simple_kind {
        PolicyKindSimple::PasswordExpiry => {
            password_expiry_policy(client, row.get("password_expiration")).await?
        }
        PolicyKindSimple::PasswordStrength => {
            password_strength_policy(client, row.get("password_strength")).await?
        }
        PolicyKindSimple::Expression => expression_policy(client, row.get("expression")).await?,
    };
    Ok(Policy {
        uid: row.get("uid"),
        slug: row.get("slug"),
        kind,
    })
}

async fn password_expiry_policy(
    client: &impl GenericClient,
    id: i32,
) -> Result<PolicyKind, StorageError> {
    let statement = client
        .prepare_cached(include_sql!("policy/password-expiration-by-id"))
        .await?;
    let row = client.query_one(&statement, &[&id]).await?;
    Ok(PolicyKind::PasswordExpiry {
        max_age: row.get("max_age"),
    })
}

async fn password_strength_policy(
    _client: &impl GenericClient,
    _id: i32,
) -> Result<PolicyKind, StorageError> {
    // let statement = client.prepare_cached(include_sql!("policy/password-strength-by-id")).await?;
    // let row = client.query_one(&statement, &[&id]).await?;
    Ok(PolicyKind::PasswordStrength {})
}
async fn expression_policy(
    client: &impl GenericClient,
    id: i32,
) -> Result<PolicyKind, StorageError> {
    let statement = client
        .prepare_cached(include_sql!("policy/expression-by-id"))
        .await?;
    let row = client.query_one(&statement, &[&id]).await?;
    Ok(PolicyKind::Expression(row.get("expression")))
}
