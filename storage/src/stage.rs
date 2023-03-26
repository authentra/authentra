use deadpool_postgres::GenericClient;
use model::{
    ConsentMode, PasswordBackend, PgConsentMode, PromptBinding, Stage, StageKind, UserField,
};
use postgres_types::FromSql;
use tokio_postgres::Row;

use crate::{include_sql, StorageError};

#[derive(Debug, FromSql)]
#[postgres(name = "stage_kind")]
enum PgStageKind {
    #[postgres(name = "deny")]
    Deny,
    #[postgres(name = "prompt")]
    Prompt,
    #[postgres(name = "identification")]
    Identification,
    #[postgres(name = "user_login")]
    UserLogin,
    #[postgres(name = "user_logout")]
    UserLogout,
    #[postgres(name = "user_write")]
    UserWrite,
    #[postgres(name = "password")]
    Password,
    #[postgres(name = "consent")]
    Consent,
}

pub(crate) async fn from_row(client: &impl GenericClient, row: Row) -> Result<Stage, StorageError> {
    let uid = row.get("uid");
    let simple_kind: PgStageKind = row.get("kind");
    let kind = match simple_kind {
        PgStageKind::Deny => StageKind::Deny,
        PgStageKind::Prompt => prompt_stage(client, uid).await?,
        PgStageKind::Identification => identification_stage(&row).await?,
        PgStageKind::UserLogin => StageKind::UserLogin,
        PgStageKind::UserLogout => StageKind::UserWrite,
        PgStageKind::UserWrite => StageKind::UserWrite,
        PgStageKind::Password => password_stage(&row).await?,
        PgStageKind::Consent => consent_stage(&row).await?,
    };
    Ok(Stage {
        uid,
        slug: row.get("slug"),
        kind,
        timeout: row.get("timeout"),
    })
}

async fn identification_stage(row: &Row) -> Result<StageKind, StorageError> {
    let password_stage: Option<i32> = row.get("identification_password_stage");
    let user_fields: Vec<UserField> = row.get("identification_fields");
    Ok(StageKind::Identification {
        password_stage,
        user_fields,
    })
}
async fn password_stage(_row: &Row) -> Result<StageKind, StorageError> {
    //TODO: Make Password backend configurable
    Ok(StageKind::Password {
        backends: vec![PasswordBackend::Internal],
    })
}

async fn consent_stage(row: &Row) -> Result<StageKind, StorageError> {
    let mode: PgConsentMode = row.get("consent_mode");
    let mode = match mode {
        PgConsentMode::Always => ConsentMode::Always,
        PgConsentMode::Once => ConsentMode::Once,
        PgConsentMode::Until => ConsentMode::Until {
            duration: row.get("consent_until"),
        },
    };
    Ok(StageKind::Consent { mode })
}

async fn prompt_stage(
    client: &impl GenericClient,
    stage_id: i32,
) -> Result<StageKind, StorageError> {
    let statement = client
        .prepare_cached(include_sql!("stage/prompt-bindings-by-stage"))
        .await?;
    let mut bindings: Vec<PromptBinding> = client
        .query(&statement, &[&stage_id])
        .await?
        .into_iter()
        .map(|row| PromptBinding {
            order: row.get("ordering"),
            prompt: row.get("prompt"),
        })
        .collect();
    bindings.sort_by_key(|v| v.order);
    Ok(StageKind::Prompt { bindings })
}
