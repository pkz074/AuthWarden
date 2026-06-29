use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::user::{NewUser, User},
};

pub async fn create_user(pool: &PgPool, new_user: NewUser) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash)
        VALUES ($1, $2)
        RETURNING id, email, password_hash, created_at
        "#,
    )
    .bind(new_user.email)
    .bind(new_user.password_hash)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(user)
}
