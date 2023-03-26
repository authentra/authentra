use deadpool_postgres::GenericClient;
use model::{Flow, FlowBinding, FlowBindingKind, FlowEntry};
use tokio_postgres::{Row, Statement};
use uuid::Uuid;

use crate::{include_sql, StorageError};

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
        kind = FlowBindingKind::Policy(policy)
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
        stage: row.get("stage"),
    })
}
