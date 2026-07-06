use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{errors::AppError, services::token as token_service, state::AppState};

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = authorization
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = token_service::decode_access_token(token, &state.jwt_secret)?;

        Ok(Self {
            user_id: claims.sub,
            email: claims.email,
        })
    }
}
