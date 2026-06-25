use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
}

// TODO
