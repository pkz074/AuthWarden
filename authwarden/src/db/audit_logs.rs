use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::audit_log::{AuditLog, NewAuditLog},
};

pub async fn create_audit_log(pool: &PgPool, new_log: NewAuditLog) -> Result<AuditLog, AppError> {
    let audit_log = sqlx::query_as::<_, AuditLog>(
        r#"
        INSERT INTO audit_logs (user_id, event_type)
        VALUES ($1, $2)
        RETURNING id, user_id, event_type, created_at
        "#,
    )
    .bind(new_log.user_id)
    .bind(new_log.event_type)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(audit_log)
}
