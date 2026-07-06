use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct NewAuditLog {
    pub user_id: Option<Uuid>,
    pub event_type: String,
}
