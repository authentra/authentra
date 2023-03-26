use async_trait::async_trait;
use datacache::DataQueryExecutor;
use deadpool_postgres::GenericClient;
use model::{Tenant, TenantQuery};
use tokio_postgres::Row;

use crate::{include_sql, StorageError};

datacache::storage!(pub TenantStorage(TenantExecutor, Tenant), id(uid: i32), unique(host: String), fields());

crate::executor!(pub TenantExecutor);

#[async_trait]
impl DataQueryExecutor<Tenant> for TenantExecutor {
    type Error = StorageError;
    type Id = i32;

    fn get_id(&self, data: &Tenant) -> Self::Id {
        data.uid
    }

    async fn find_one(&self, query: &TenantQuery) -> Result<Tenant, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            TenantQuery::uid(uid) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("tenant/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            TenantQuery::host(host) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("tenant/by-host")).await?,
                    &[&host],
                )
                .await?
            }
        };
        Ok(from_row(row))
    }
    async fn find_all_ids(
        &self,
        query: Option<&TenantQuery>,
    ) -> Result<Vec<Self::Id>, Self::Error> {
        if let Some(query) = query {
            match query {
                TenantQuery::uid(id) => return Ok(vec![*id]),
                TenantQuery::host(_host) => todo!(),
            }
        } else {
            let conn = self.get_conn().await?;
            let statement = conn.prepare_cached(include_sql!("tenant/all-ids")).await?;
            let ids = conn.query(&statement, &[]).await?;
            Ok(ids.into_iter().map(|row| row.get("uid")).collect())
        }
    }
    async fn find_optional(&self, query: &TenantQuery) -> Result<Option<Tenant>, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            TenantQuery::uid(uid) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("tenant/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            TenantQuery::host(host) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("tenant/by-host")).await?,
                    &[&host],
                )
                .await?
            }
        };
        Ok(row.map(from_row))
    }
    async fn delete(&self, _data: &TenantQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
}

pub(crate) fn from_row(row: Row) -> Tenant {
    Tenant {
        uid: row.get("uid"),
        host: row.get("host"),
        default: row.get("is_default"),
        title: row.get("title"),
        logo: row.get("logo"),
        favicon: row.get("favicon"),
        invalidation_flow: row.get("invalidation_flow"),
        authentication_flow: row.get("authentication_flow"),
        authorization_flow: row.get("authorization_flow"),
        enrollment_flow: row.get("enrollment_flow"),
        recovery_flow: row.get("recovery_flow"),
        unenrollment_flow: row.get("unenrollment_flow"),
        configuration_flow: row.get("configuration_flow"),
    }
}
