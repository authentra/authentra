use rhai::{def_package, plugin::*};

// #[serde(skip)]
// pub uid: Uuid,
// pub name: String,
// pub avatar_url: String,
// #[serde(skip)]
// pub authenticated: bool,

def_package! {
    pub UserPackage(module) {
        combine_with_exported_module!(module, "User", user_module);
    }
}

#[export_module]
mod user_module {
    use authust_model::user::PartialUser;
    use authust_model::PendingUser;

    #[rhai_fn(get = "uid", pure)]
    pub fn get_uid_pending(obj: &mut PendingUser) -> ImmutableString {
        obj.uid.to_string().into()
    }

    #[rhai_fn(get = "name", pure)]
    pub fn get_name_pending(obj: &mut PendingUser) -> ImmutableString {
        obj.name.clone().into()
    }
    #[rhai_fn(global, pure, get = "avatar_url")]
    pub fn get_avatar_url_pending(obj: &mut PendingUser) -> Option<ImmutableString> {
        obj.avatar_url.clone().map(Into::into)
    }
    #[rhai_fn(get = "authenticated", pure)]
    pub fn get_authenticated_pending(obj: &mut PendingUser) -> bool {
        obj.authenticated
    }

    #[rhai_fn(global, pure, get = "uid")]
    pub fn get_uid_partial(obj: &mut PartialUser) -> ImmutableString {
        obj.uid.to_string().into()
    }

    #[rhai_fn(global, pure, get = "name")]
    pub fn get_name_partial(obj: &mut PartialUser) -> ImmutableString {
        obj.name.clone().into()
    }
    #[rhai_fn(global, pure, get = "avatar_url")]
    pub fn get_avatar_url_partial(obj: &mut PartialUser) -> Option<ImmutableString> {
        obj.avatar_url.clone().map(Into::into)
    }
    #[rhai_fn(get = "authenticated", pure)]
    pub fn get_is_admin_partial(obj: &mut PartialUser) -> bool {
        obj.is_admin
    }
}
