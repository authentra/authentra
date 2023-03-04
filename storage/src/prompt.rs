use async_trait::async_trait;
use datacache::{Data, DataQueryExecutor};
use model::{Prompt, PromptQuery};
use tokio_postgres::Row;

use crate::{include_sql, StorageError};

datacache::storage!(pub PromptStorage(PromptExecutor, Prompt), id(uid: i32), unique(), fields());

crate::executor!(pub PromptExecutor);

#[async_trait]
impl DataQueryExecutor<Prompt> for PromptExecutor {
    type Error = StorageError;
    type Id = i32;

    fn get_id(&self, data: &Prompt) -> Self::Id {
        data.uid
    }

    async fn find_one(&self, query: PromptQuery) -> Result<Prompt, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            PromptQuery::uid(uid) => {
                conn.query_one(
                    &conn.prepare_cached(include_sql!("prompt/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
        };
        Ok(from_row(row))
    }
    async fn find_all_ids(&self, query: PromptQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
    async fn find_optional(&self, query: PromptQuery) -> Result<Option<Prompt>, Self::Error> {
        let conn = self.get_conn().await?;
        let row = match query {
            PromptQuery::uid(uid) => {
                conn.query_opt(
                    &conn.prepare_cached(include_sql!("prompt/by-id")).await?,
                    &[&uid],
                )
                .await?
            }
        };
        Ok(row.map(from_row))
    }
    async fn create(&self, data: Data<Prompt>) -> Result<(), Self::Error> {
        todo!()
    }
    async fn update(&self, data: Data<Prompt>) -> Result<(), Self::Error> {
        todo!()
    }
    async fn delete(&self, data: PromptQuery) -> Result<Vec<Self::Id>, Self::Error> {
        todo!()
    }
}

fn from_row(row: Row) -> Prompt {
    Prompt {
        uid: row.get("uid"),
        field_key: row.get("field_key"),
        label: row.get("label"),
        kind: row.get("kind"),
        placeholder: row.get("placeholder"),
        required: row.get("required"),
        help_text: row.get("help_text"),
    }
}
