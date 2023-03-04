use async_trait::async_trait;
use datacache::{Data, DataQueryExecutor};
use deadpool_postgres::GenericClient;
use model::{
    ConsentMode, PasswordBackend, PgConsentMode, PromptBinding, Reference, Stage, StageKind,
    StageQuery, UserField,
};
use postgres_types::FromSql;
use tokio_postgres::Row;

use crate::{include_sql, StorageError};

datacache::storage!(pub StageStorage(StageExecutor, Stage), id(uid: i32), unique(slug: String), fields());

crate::executor!(pub StageExecutor);

#[async_trait]
impl DataQueryExecutor<Stage> for StageExecutor {
    type Error = StorageError;
    type Id = i32;

    fn get_id(&self, data: &Stage) -> Self::Id {
        data.uid
    }

    async fn find_one(&self, query: StageQuery) -> Result<Stage, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            StageQuery::uid(uid) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("stage/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            StageQuery::slug(slug) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("stage/by-slug")).await?,
                    &[&slug],
                )
                .await?
            }
        };
        from_row(&conn, row).await
    }
    async fn find_all_ids(&self, query: StageQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
    async fn find_optional(&self, query: StageQuery) -> Result<Option<Stage>, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            StageQuery::uid(uid) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("stage/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
            StageQuery::slug(slug) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("stage/by-slug")).await?,
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
    async fn create(&self, data: Data<Stage>) -> Result<(), Self::Error> {
        todo!()
    }
    async fn update(&self, data: Data<Stage>) -> Result<(), Self::Error> {
        todo!()
    }
    async fn delete(&self, data: StageQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
}
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

async fn from_row(client: &impl GenericClient, row: Row) -> Result<Stage, StorageError> {
    let uid = row.get("uid");
    let simple_kind: PgStageKind = row.get("kind");
    let kind = match simple_kind {
        PgStageKind::Deny => StageKind::Deny,
        PgStageKind::Prompt => prompt_stage(client, uid).await?,
        PgStageKind::Identification => identification_stage(client, &row).await?,
        PgStageKind::UserLogin => StageKind::UserLogin,
        PgStageKind::UserLogout => StageKind::UserWrite,
        PgStageKind::UserWrite => StageKind::UserWrite,
        PgStageKind::Password => password_stage(client, &row).await?,
        PgStageKind::Consent => consent_stage(client, &row).await?,
    };
    Ok(Stage {
        uid,
        slug: row.get("slug"),
        kind,
        timeout: row.get("timeout"),
    })
}

async fn identification_stage(
    client: &impl GenericClient,
    row: &Row,
) -> Result<StageKind, StorageError> {
    let password_stage_id: Option<i32> = row.get("identification_password_stage");
    let identification_id: i32 = row.get("identification_stage");
    let statement = client
        .prepare_cached(include_sql!("stage/identification-by-id"))
        .await?;
    let id_row = client.query_one(&statement, &[&identification_id]).await?;
    let user_fields: Vec<UserField> = id_row.get("user_fields");
    Ok(StageKind::Identification {
        password: password_stage_id.map(Reference::new_uid),
        user_fields,
    })
}
async fn password_stage(client: &impl GenericClient, row: &Row) -> Result<StageKind, StorageError> {
    //TODO: Make Password backend configurable
    Ok(StageKind::Password {
        backends: vec![PasswordBackend::Internal],
    })
}

async fn consent_stage(client: &impl GenericClient, row: &Row) -> Result<StageKind, StorageError> {
    let statement = client
        .prepare_cached(include_sql!("stage/consent-by-id"))
        .await?;
    let consent_id: i32 = row.get("consent_stage");
    let consent_row = client.query_one(&statement, &[&consent_id]).await?;
    let mode: PgConsentMode = consent_row.get("mode");
    let mode = match mode {
        PgConsentMode::Always => ConsentMode::Always,
        PgConsentMode::Once => ConsentMode::Once,
        PgConsentMode::Until => ConsentMode::Until {
            duration: consent_row.get("until"),
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
            prompt: Reference::new_uid(row.get("prompt")),
        })
        .collect();
    bindings.sort_by_key(|v| v.order);
    Ok(StageKind::Prompt { bindings })
}
