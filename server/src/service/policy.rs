mod service;

use http::Uri;
use once_cell::sync::Lazy;
use policy_engine::{
    context::RhaiContext,
    request::RhaiRequest,
    rhai::Scope,
    uri::{RhaiUri, Scheme},
};
pub use service::PolicyService;

use crate::{
    api::ExecutorQuery,
    executor::flow::{CheckContextData, CheckContextRequest},
};

pub static DUMMY_SCOPE: Lazy<Scope> = Lazy::new(|| {
    let uri = Uri::from_static("http://host/");
    let ctx = CheckContextData {
        request: CheckContextRequest {
            uri,
            host: "host".into(),
            scheme: Scheme::Http,
            query: ExecutorQuery::default(),
            user: None,
        },
        pending_user: None,
    };

    create_scope(&ctx)
});

pub fn create_scope(context: &CheckContextData) -> Scope<'static> {
    let mut scope = Scope::new();
    let uri = RhaiUri::create(
        context.request.scheme.clone(),
        context.request.host.clone(),
        &context.request.uri,
    )
    .unwrap();
    let req = RhaiRequest {
        uri,
        user: context.request.user.clone(),
    };
    let ctx = RhaiContext {
        pending_user: context.pending_user.clone(),
    };
    scope.push_constant("request", req);
    scope.push_constant("context", ctx);
    scope
}
