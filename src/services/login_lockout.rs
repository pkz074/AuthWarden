use redis::{AsyncCommands, Client};

use crate::errors::AppError;

pub const LOGIN_LOCKOUT_MAX_FAILURES: u64 = 5;
pub const LOGIN_LOCKOUT_FAILURE_WINDOW_SECONDS: u64 = 15 * 60;
pub const LOGIN_LOCKOUT_SECONDS: u64 = 15 * 60;

pub async fn is_login_locked(redis: &Client, email: &str) -> Result<bool, AppError> {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let locked = connection
        .exists(lockout_key(email))
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(locked)
}

pub async fn record_failed_login(redis: &Client, email: &str) -> Result<(), AppError> {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let key = failure_key(email);
    let failure_count: u64 = connection
        .incr(&key, 1_u8)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if failure_count == 1 {
        let _: bool = connection
            .expire(&key, LOGIN_LOCKOUT_FAILURE_WINDOW_SECONDS as i64)
            .await
            .map_err(|_| AppError::InternalServerError)?;
    }

    if failure_count >= LOGIN_LOCKOUT_MAX_FAILURES {
        let _: () = connection
            .set_ex(lockout_key(email), "1", LOGIN_LOCKOUT_SECONDS)
            .await
            .map_err(|_| AppError::InternalServerError)?;
    }

    Ok(())
}

pub async fn clear_login_failures(redis: &Client, email: &str) -> Result<(), AppError> {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let keys = [failure_key(email), lockout_key(email)];
    let _: () = connection
        .del(&keys)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(())
}

fn failure_key(email: &str) -> String {
    format!("login_failures:{}", email_key(email))
}

fn lockout_key(email: &str) -> String {
    format!("login_lockout:{}", email_key(email))
}

fn email_key(email: &str) -> String {
    email.replace([':', ' '], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockout_keys_escape_unsafe_characters() {
        assert_eq!(
            failure_key(" test:user@example.com "),
            "login_failures:_test_user@example.com_"
        );
        assert_eq!(
            lockout_key(" test:user@example.com "),
            "login_lockout:_test_user@example.com_"
        );
    }
}
