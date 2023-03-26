use model::{Policy, PolicyKind, PolicyKindSimple};
use tokio_postgres::Row;

use crate::StorageError;

pub(crate) fn from_row(row: Row) -> Result<Policy, StorageError> {
    let simple_kind: PolicyKindSimple = row.get("kind");
    let kind = match simple_kind {
        PolicyKindSimple::PasswordExpiry => password_expiry_policy(&row)?,
        PolicyKindSimple::PasswordStrength => password_strength_policy(&row)?,
        PolicyKindSimple::Expression => expression_policy(&row)?,
    };
    Ok(Policy {
        uid: row.get("uid"),
        slug: row.get("slug"),
        kind,
    })
}

fn password_expiry_policy(row: &Row) -> Result<PolicyKind, StorageError> {
    Ok(PolicyKind::PasswordExpiry {
        max_age: row.get("password_max_age"),
    })
}

fn password_strength_policy(_row: &Row) -> Result<PolicyKind, StorageError> {
    // let statement = client.prepare_cached(include_sql!("policy/password-strength-by-id")).await?;
    // let row = client.query_one(&statement, &[&id]).await?;
    Ok(PolicyKind::PasswordStrength {})
}
fn expression_policy(row: &Row) -> Result<PolicyKind, StorageError> {
    Ok(PolicyKind::Expression(row.get("expression")))
}
