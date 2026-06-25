use crate::errors::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    // TODO

    let _ = password;

    Ok("TODO_REPLACE_WITH_ARGON2_HASH".to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    // TODO

    let _ = (password, password_hash);

    Ok(false)
}
