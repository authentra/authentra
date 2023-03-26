use std::sync::Arc;

use moka::sync::Cache;
use policy_engine::{compile, rhai::AST};
use storage::Storage;

use super::DUMMY_SCOPE;

#[derive(Clone)]
#[repr(transparent)]
pub struct PolicyService(Arc<InternalPolicyService>);

impl PolicyService {
    pub fn new(storage: Storage) -> Self {
        Self(Arc::new(InternalPolicyService {
            storage,
            asts: Cache::builder().build(),
        }))
    }

    pub async fn get_ast(&self, id: i32) -> Option<Arc<AST>> {
        self.0.get_ast(id).await
    }

    pub async fn invalidate(&self, policy: i32) {
        self.0.invalidate(policy).await;
    }
}

struct InternalPolicyService {
    storage: Storage,
    asts: Cache<i32, Option<Arc<AST>>>,
}

impl InternalPolicyService {
    pub async fn get_ast(&self, id: i32) -> Option<Arc<AST>> {
        let policy = match self.storage.get_policy(id).await {
            Ok(opt) => {
                opt?
            }
            Err(err) => {
                tracing::error!(
                    error = ?err,
                    "An error occurred while getting ast for policy {id}!"
                );
                return None;
            }
        };
        self.asts
            .optionally_get_with(policy.uid, move || match &policy.kind {
                model::PolicyKind::Expression(expr) => {
                    let compiled = compile(expr, &DUMMY_SCOPE);
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

    pub async fn invalidate(&self, policy: i32) {
        self.asts.invalidate(&policy);
    }
}
