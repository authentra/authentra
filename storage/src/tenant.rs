use async_trait::async_trait;
use datacache::{DataQueryExecutor, DataRef};
use deadpool_postgres::GenericClient;
use model::{FlowQuery, Tenant, TenantQuery};
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
                TenantQuery::uid(id) => return Ok(vec![id.clone()]),
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

fn from_row(row: Row) -> Tenant {
    Tenant {
        uid: row.get("uid"),
        host: row.get("host"),
        default: row.get("is_default"),
        title: row.get("title"),
        logo: row.get("logo"),
        favicon: row.get("favicon"),
        invalidation_flow: row
            .get::<_, Option<i32>>("invalidation_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
        authentication_flow: row
            .get::<_, Option<i32>>("authentication_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
        authorization_flow: row
            .get::<_, Option<i32>>("authorization_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
        enrollment_flow: row
            .get::<_, Option<i32>>("enrollment_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
        recovery_flow: row
            .get::<_, Option<i32>>("recovery_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
        unenrollment_flow: row
            .get::<_, Option<i32>>("unenrollment_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
        configuration_flow: row
            .get::<_, Option<i32>>("configuration_flow")
            .map(|uid| DataRef::new(FlowQuery::uid(uid))),
    }
}
