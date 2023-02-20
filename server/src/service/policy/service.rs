use std::sync::Arc;

use deadpool_postgres::{GenericClient, Pool};
use model::{PartialPolicy, Policy, Reference};
use moka::sync::Cache;
use policy_engine::{compile, rhai::AST};

use crate::flow_storage::{FlowStorage, ReferenceLookup};

use super::DUMMY_SCOPE;

#[derive(Clone)]
#[repr(transparent)]
pub struct PolicyService(Arc<InternalPolicyService>);

impl PolicyService {
    pub fn new(storage: FlowStorage, pool: Pool) -> Self {
        Self(Arc::new(InternalPolicyService {
            storage,
            pool,
            asts: Cache::builder().build(),
        }))
    }

    pub async fn get_ast(&self, policy: Reference<Policy>) -> Option<Arc<AST>> {
        self.0.get_ast(policy).await
    }

    pub async fn invalidate(&self, policy: i32) {
        self.0.invalidate(policy).await;
    }

    pub async fn list_all<C: GenericClient>(
        &self,
        client: &C,
    ) -> Result<Vec<PartialPolicy>, tokio_postgres::Error> {
        self.0.list_all(client).await
    }
}

struct InternalPolicyService {
    storage: FlowStorage,
    pool: Pool,
    asts: Cache<i32, Option<Arc<AST>>>,
}

impl InternalPolicyService {
    pub async fn get_ast(&self, policy: Reference<Policy>) -> Option<Arc<AST>> {
        let Some(policy) = self.storage.lookup_reference(&policy).await else { return None};
        self.asts
            .optionally_get_with(policy.uid, move || match &policy.kind {
                model::PolicyKind::Expression(expr) => {
                    let compiled = compile(&expr, &DUMMY_SCOPE);
                    Some(match compiled {
                        Ok(ast) => Some(Arc::new(ast)),
                        Err(err) => {
                            tracing::warn!(policy = ?policy, "Failed to compile policy! {err}");
                            None
                        }
                    })
                    // Some(Arc::new(compile(&expr, &DUMMY_SCOPE)))
                }
                _ => None,
            })
            .flatten()
    }

    pub async fn list_all_references<C: GenericClient>(
        &self,
        client: &C,
    ) -> Result<Vec<Reference<Policy>>, tokio_postgres::Error> {
        let statement = client.prepare_cached("select uid from policies").await?;
        let records: Vec<Reference<Policy>> = client
            .query(&statement, &[])
            .await?
            .into_iter()
            .map(|rec| Reference::<Policy>::new_uid(rec.get(0)))
            .collect();
        Ok(records)
    }

    pub async fn list_all<C: GenericClient>(
        &self,
        client: &C,
    ) -> Result<Vec<PartialPolicy>, tokio_postgres::Error> {
        let references = self.list_all_references(client).await?;
        let mut policies = Vec::new();
        for reference in references {
            if let Some(policy) = self.storage.lookup_reference(&reference).await {
                policies.push(policy.as_ref().into());
            }
        }
        Ok(policies)
    }

    pub async fn invalidate(&self, policy: i32) {
        self.asts.invalidate(&policy);
        self.storage
            .delete_ref(&Reference::<Policy>::new_uid(policy))
            .await;
    }
}
