use crate::errors::AppError;
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString, rand_core::OsRng},
};

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::InternalServerError)
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|_| AppError::InternalServerError)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_matching_password() {
        let hash = hash_password("Password123").unwrap();

        assert!(verify_password("Password123", &hash).unwrap());
    }

    #[test]
    fn rejects_wrong_password() {
        let hash = hash_password("Password123").unwrap();

        assert!(!verify_password("WrongPassword123", &hash).unwrap());
    }

    #[test]
    fn rejects_malformed_password_hash() {
        let result = verify_password("Password123", "not-a-password-hash");

        assert!(result.is_err());
    }
}
