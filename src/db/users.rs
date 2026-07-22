use sqlx::{Error as SqlxError, PgPool};

use crate::{
    errors::AppError,
    models::user::{NewUser, User},
};
use uuid::Uuid;

pub async fn create_user(pool: &PgPool, new_user: NewUser) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash)
        VALUES ($1, $2)
        RETURNING id, email, password_hash
        "#,
    )
    .bind(new_user.email)
    .bind(new_user.password_hash)
    .fetch_one(pool)
    .await
    .map_err(map_create_user_error)?;

    Ok(user)
}

fn map_create_user_error(error: SqlxError) -> AppError {
    if let SqlxError::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
    {
        return AppError::Conflict("email is already registered".to_string());
    }

    AppError::InternalServerError
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, password_hash
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, password_hash
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(user)
}
