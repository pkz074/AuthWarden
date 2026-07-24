use redis::{AsyncCommands, Client};

use crate::{errors::AppError, services::refresh_token};

pub const OAUTH_STATE_TTL_SECONDS: u64 = 600;

fn oauth_state_key(state: &str) -> String {
    format!("oauth_state:{state}")
}

pub fn generate_oauth_state() -> String {
    refresh_token::generate_refresh_token()
}

pub async fn store_oauth_state(
    redis: &Client,
    state: &str,
    provider: &str,
) -> Result<(), AppError> {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let _: () = connection
        .set_ex(oauth_state_key(state), provider, OAUTH_STATE_TTL_SECONDS)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(())
}

pub async fn validate_oauth_state(
    redis: &Client,
    state: &str,
    expected_provider: &str,
) -> Result<(), AppError> {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let stored_provider: Option<String> = connection
        .get(oauth_state_key(state))
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let _: () = connection
        .del(oauth_state_key(state))
        .await
        .map_err(|_| AppError::InternalServerError)?;

    match stored_provider {
        Some(provider) if provider == expected_provider => Ok(()),
        _ => Err(AppError::Unauthorized),
    }
}

#[cfg(test)]
mod tests {
    use super::generate_oauth_state;

    #[test]
    fn generated_states_are_distinct_and_url_safe() {
        let first = generate_oauth_state();
        let second = generate_oauth_state();

        assert_eq!(first.len(), 43);
        assert_ne!(first, second);
        assert!(
            first
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        );
    }
}
