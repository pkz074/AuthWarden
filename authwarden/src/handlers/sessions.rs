use axum::{
    Json,
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

use crate::{
    errors::AppError,
    models::session::{NewRefreshSession, RefreshForm, TokenPair},
    services::{refresh_token as refresh_tokens, token},
    state::AppState,
};

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RefreshForm>,
) -> Result<Response, AppError> {
    let token_hash = refresh_tokens::hash_refresh_token(&form.refresh_token)?;
    let session = crate::db::sessions::find_session_by_token_hash(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if session.revoked_at.is_some() || session.expires_at <= OffsetDateTime::now_utc() {
        return Err(AppError::Unauthorized);
    }

    let user = crate::db::users::find_user_by_id(&state.db, session.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    crate::db::sessions::revoke_session(&state.db, session.id).await?;

    let access_token = token::issue_access_token(&user, &state.jwt_secret)?;
    let refresh_token = refresh_tokens::generate_refresh_token();
    let replacement_hash = refresh_tokens::hash_refresh_token(&refresh_token)?;
    let replacement_session = NewRefreshSession {
        user_id: user.id,
        token_hash: replacement_hash,
        expires_at: OffsetDateTime::now_utc() + Duration::days(30),
    };

    crate::db::sessions::create_session(&state.db, replacement_session).await?;

    Ok(Json(TokenPair {
        access_token,
        refresh_token,
    })
    .into_response())
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RefreshForm>,
) -> Result<Response, AppError> {
    let token_hash = refresh_tokens::hash_refresh_token(&form.refresh_token)?;
    let session = crate::db::sessions::find_session_by_token_hash(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    crate::db::sessions::revoke_session(&state.db, session.id).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
