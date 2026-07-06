use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::session::{NewRefreshSession, RefreshSession},
};

pub async fn create_session(
    pool: &PgPool,
    new_session: NewRefreshSession,
) -> Result<RefreshSession, AppError> {
    let session = sqlx::query_as::<_, RefreshSession>(
        r#"
        INSERT INTO refresh_sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, token_hash, expires_at, revoked_at, created_at
        "#,
    )
    .bind(new_session.user_id)
    .bind(new_session.token_hash)
    .bind(new_session.expires_at)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(session)
}

pub async fn find_session_by_token_hash(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshSession>, AppError> {
    let session = sqlx::query_as::<_, RefreshSession>(
        r#"
        SELECT id, user_id, token_hash, expires_at, revoked_at, created_at
        FROM refresh_sessions
        WHERE token_hash = $1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(session)
}

pub async fn revoke_session(pool: &PgPool, session_id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"
        UPDATE refresh_sessions
        SET revoked_at = NOW()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    if result.rows_affected() == 0 {
        return Err(AppError::Unauthorized);
    }

    Ok(())
}
