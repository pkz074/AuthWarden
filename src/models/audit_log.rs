use uuid::Uuid;

#[derive(Debug)]
pub struct NewAuditLog {
    pub user_id: Option<Uuid>,
    pub event_type: String,
}
