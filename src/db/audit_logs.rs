use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, models::audit_log::NewAuditLog};

pub async fn create_audit_log(pool: &PgPool, new_log: NewAuditLog) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (user_id, event_type)
        VALUES ($1, $2)
        "#,
    )
    .bind(new_log.user_id)
    .bind(new_log.event_type)
    .execute(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(())
}

pub async fn record_auth_event(pool: &PgPool, user_id: Option<Uuid>, event_type: &str) {
    let new_log = NewAuditLog {
        user_id,
        event_type: event_type.to_string(),
    };

    if let Err(error) = create_audit_log(pool, new_log).await {
        tracing::warn!(?error, event_type, "failed to write audit log");
    }
}
