use crate::errors::AppError;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};

pub fn generate_refresh_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);

    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> Result<String, AppError> {
    let digest = Sha256::digest(token.as_bytes());

    Ok(URL_SAFE_NO_PAD.encode(digest))
}

#[cfg(test)]
mod tests {
    use super::{generate_refresh_token, hash_refresh_token};

    #[test]
    fn generated_tokens_are_distinct_and_url_safe() {
        let first = generate_refresh_token();
        let second = generate_refresh_token();

        assert_eq!(first.len(), 43);
        assert_ne!(first, second);
        assert!(
            first
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        );
    }

    #[test]
    fn hashing_is_deterministic_and_does_not_return_the_token() {
        let token = generate_refresh_token();
        let first_hash = hash_refresh_token(&token).unwrap();
        let second_hash = hash_refresh_token(&token).unwrap();

        assert_eq!(first_hash, second_hash);
        assert_ne!(first_hash, token);
    }

    #[test]
    fn different_tokens_have_different_hashes() {
        let first_hash = hash_refresh_token(&generate_refresh_token()).unwrap();
        let second_hash = hash_refresh_token(&generate_refresh_token()).unwrap();

        assert_ne!(first_hash, second_hash);
    }
}
