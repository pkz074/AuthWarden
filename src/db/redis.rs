use redis::{AsyncCommands, Client};
use time::OffsetDateTime;

use crate::errors::AppError;

fn revoked_refresh_key(token_hash: &str) -> String {
    format!("revoked_refresh_token:{token_hash}")
}

pub async fn is_refresh_token_revoked(redis: &Client, token_hash: &str) -> Result<bool, AppError> {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let exists = connection
        .exists(revoked_refresh_key(token_hash))
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(exists)
}

pub async fn mark_refresh_token_revoked(
    redis: &Client,
    token_hash: &str,
    expires_at: OffsetDateTime,
) -> Result<(), AppError> {
    let ttl = (expires_at - OffsetDateTime::now_utc()).whole_seconds();

    if ttl <= 0 {
        return Ok(());
    }

    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let _: () = connection
        .set_ex(revoked_refresh_key(token_hash), "1", ttl as u64)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(())
}
