use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
}
