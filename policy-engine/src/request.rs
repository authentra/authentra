use crate::uri::RhaiUri;
use authust_model::user::PartialUser;
use rhai::{def_package, plugin::*};

#[derive(Clone)]
pub struct RhaiRequest {
    pub uri: RhaiUri,
    pub user: Option<PartialUser>,
}

def_package! {
    pub RequestPackage(module) {
        combine_with_exported_module!(module, "Request", request_module);
    }
}

#[export_module]
mod request_module {
    use crate::uri::RhaiUri;

    #[rhai_fn(global, pure, get = "uri")]
    pub fn get_uri(req: &mut RhaiRequest) -> RhaiUri {
        req.uri.clone()
    }
    #[rhai_fn(global, pure, get = "user")]
    pub fn get_user(req: &mut RhaiRequest) -> Option<PartialUser> {
        req.user.clone()
    }
}
