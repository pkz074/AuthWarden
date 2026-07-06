use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use time::{Duration, OffsetDateTime};

use crate::{errors::AppError, models::auth::Claims, models::user::User};

pub fn issue_access_token(user: &User, secret: &str) -> Result<String, AppError> {
    let expires_at = OffsetDateTime::now_utc() + Duration::minutes(15);
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        exp: expires_at.unix_timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::InternalServerError)
}

pub fn decode_access_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|token_data| token_data.claims)
        .map_err(|_| AppError::Unauthorized)
}
