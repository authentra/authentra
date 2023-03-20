use async_trait::async_trait;
use datacache::{DataQueryExecutor, DataRef, LookupRef};
use deadpool_postgres::GenericClient;
use model::{Flow, FlowBinding, FlowBindingKind, FlowEntry, FlowQuery, PolicyQuery, StageQuery};
use tokio_postgres::{Row, Statement};
use uuid::Uuid;

use crate::{include_sql, ProxiedStorage, ReverseLookup, StorageError};

datacache::storage!(pub FlowStorage(FlowExecutor, Flow), id(uid: i32), unique(slug: String), fields());

crate::executor!(pub FlowExecutor);

#[async_trait]
impl DataQueryExecutor<Flow> for FlowExecutor {
    type Error = StorageError;

    type Id = i32;

    fn get_id(&self, data: &Flow) -> Self::Id {
        data.uid
    }

    async fn find_one(&self, query: &FlowQuery) -> Result<Flow, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            FlowQuery::uid(uid) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("flow/by-id")).await?,
                    &[uid],
                )
                .await?
            }
            FlowQuery::slug(slug) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("flow/by-slug")).await?,
                    &[slug],
                )
                .await?
            }
        };
        Ok(from_row(&conn, row).await?)
    }
    async fn find_all_ids(&self, query: Option<&FlowQuery>) -> Result<Vec<Self::Id>, Self::Error> {
        if let Some(query) = query {
            match query {
                FlowQuery::uid(id) => return Ok(vec![id.clone()]),
                FlowQuery::slug(_slug) => todo!(),
            }
        } else {
            let conn = self.get_conn().await?;
            let statement = conn.prepare_cached(include_sql!("flow/all-ids")).await?;
            let ids = conn.query(&statement, &[]).await?;
            Ok(ids.into_iter().map(|row| row.get("uid")).collect())
        }
    }
    async fn find_optional(&self, query: &FlowQuery) -> Result<Option<Flow>, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            FlowQuery::uid(uid) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("flow/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            FlowQuery::slug(slug) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("flow/by-slug")).await?,
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
    async fn delete(&self, _data: &FlowQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
}

pub(crate) async fn from_row(client: &impl GenericClient, row: Row) -> Result<Flow, StorageError> {
    let binding_statement = client
        .prepare_cached(include_sql!("flow/bindings-by-flow"))
        .await?;
    let uid = row.get("uid");
    let bindings = get_bindings(client, binding_statement, uid).await?;
    let entries = get_entries(client, uid).await?;
    Ok(Flow {
        uid,
        slug: row.get("slug"),
        title: row.get("title"),
        designation: row.get("designation"),
        authentication: row.get("authentication"),
        bindings,
        entries,
    })
}

async fn get_bindings(
    client: &impl GenericClient,
    statement: Statement,
    id: i32,
) -> Result<Vec<FlowBinding>, tokio_postgres::Error> {
    let res = client.query(&statement, &[&id]).await?;
    let mut bindings: Vec<FlowBinding> = res.into_iter().map(binding_from_row).collect();
    bindings.sort_by_key(|v| v.order);
    Ok(bindings)
}

fn binding_from_row(row: Row) -> FlowBinding {
    let kind;
    if let Some(user) = row.get::<_, Option<Uuid>>("user_binding") {
        kind = FlowBindingKind::User(user);
    } else if let Some(group) = row.get::<_, Option<Uuid>>("group_binding") {
        kind = FlowBindingKind::Group(group);
    } else if let Some(policy) = row.get::<_, Option<i32>>("policy") {
        kind = FlowBindingKind::Policy(DataRef::new(PolicyQuery::uid(policy)))
    } else {
        tracing::warn!(row = ?row, "Invalid flow binding");
        kind = FlowBindingKind::User(Uuid::nil());
    }
    FlowBinding {
        enabled: row.get("enabled"),
        negate: row.get("negate"),
        order: row.get("ordering"),
        kind,
    }
}

async fn get_entries(client: &impl GenericClient, id: i32) -> Result<Vec<FlowEntry>, StorageError> {
    let statement = client
        .prepare_cached(include_sql!("flow/entries-by-flow"))
        .await?;
    let rows_iter = client.query(&statement, &[&id]).await?.into_iter();
    let mut rows = Vec::new();
    for row in rows_iter {
        rows.push(entry_from_row(client, row).await?);
    }
    Ok(rows)
}

async fn entry_from_row(client: &impl GenericClient, row: Row) -> Result<FlowEntry, StorageError> {
    let statement = client
        .prepare_cached(include_sql!("flow/bindings-by-entry"))
        .await?;
    let bindings = get_bindings(client, statement, row.get("uid")).await?;
    Ok(FlowEntry {
        ordering: row.get("ordering"),
        bindings,
        stage: DataRef::new(StageQuery::uid(row.get("stage"))),
    })
}

#[async_trait]
impl ReverseLookup<Flow> for ProxiedStorage {
    async fn reverse_lookup(&self, sub: &Flow) {
        self.lookup(&DataRef::<Flow>::new(FlowQuery::uid(sub.uid)))
            .await
            .expect("Failed to lookup flow");
        for binding in &sub.bindings {
            if let FlowBindingKind::Policy(policy) = &binding.kind {
                self.lookup(policy).await.expect("Failed to lookup policy");
            }
        }
        for entry in &sub.entries {
            for binding in &entry.bindings {
                if let FlowBindingKind::Policy(policy) = &binding.kind {
                    self.lookup(policy).await.expect("Failed to lookup policy");
                }
            }
            let stage = self
                .lookup(&entry.stage)
                .await
                .expect("Failed to lookup stage");
            self.reverse_lookup(&*stage).await;
        }
    }
}
