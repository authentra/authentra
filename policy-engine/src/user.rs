use rhai::plugin::*;

// #[serde(skip)]
// pub uid: Uuid,
// pub name: String,
// pub avatar_url: String,
// #[serde(skip)]
// pub authenticated: bool,

#[export_module]
mod user_module {

    type PendingUser = authust_model::PendingUser;

    #[rhai_fn(get = "uid", pure)]
    pub fn get_uid(obj: &mut PendingUser) -> String {
        obj.uid.to_string()
    }

    #[rhai_fn(get = "name", pure)]
    pub fn get_name(obj: &mut PendingUser) -> String {
        obj.name.clone()
    }
    #[rhai_fn(get = "authenticated", pure)]
    pub fn get_authenticated(obj: &mut PendingUser) -> bool {
        obj.authenticated.clone()
    }
}
