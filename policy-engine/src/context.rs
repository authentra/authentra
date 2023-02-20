use authust_model::PendingUser;
use rhai::{
    def_package,
    packages::{ArithmeticPackage, BasicMathPackage, BasicStringPackage, LanguageCorePackage},
    plugin::*,
};

use crate::{network::NetworkPackage, request::RequestPackage, user::UserPackage};

#[derive(Debug, Clone)]
pub struct RhaiContext {
    pub pending_user: Option<PendingUser>,
}

def_package! {
    pub ContextPackage(module): UserPackage, NetworkPackage, RequestPackage, LanguageCorePackage, ArithmeticPackage, BasicMathPackage, BasicStringPackage {
        combine_with_exported_module!(module, "Context", context_module);
    }
}

#[export_module]
mod context_module {
    #[rhai_fn(global, pure, get = "pending_user")]
    pub fn get_pending_user(context: &mut RhaiContext) -> Option<PendingUser> {
        context.pending_user.clone()
    }
}
