use uuid::Uuid;

pub struct Session {
    pub session_id: String,
    pub user_id: Option<Uuid>,
}
