mod service;

use http::Uri;
use once_cell::sync::Lazy;
use policy_engine::{
    request::RhaiRequest,
    rhai::{Scope, AST},
    uri::RhaiUri,
    ExpressionCompilationError,
};
pub use service::PolicyService;
pub type ASTResult = Result<AST, ExpressionCompilationError>;

pub static DUMMY_SCOPE: Lazy<Scope> = Lazy::new(|| {
    let mut scope = Scope::new();
    let uri = RhaiUri::from_uri(&Uri::from_static("http://host/")).unwrap();
    let req = RhaiRequest { uri, user: None };
    scope.push_constant("request", req);
    scope
});
