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

    match crate::db::redis::is_refresh_token_revoked(&state.redis, &token_hash).await {
        Ok(true) => return Err(AppError::Unauthorized),
        Ok(false) => {}
        Err(error) => {
            tracing::warn!(?error, "failed to check revoked refresh token cache");
        }
    }

    let session = crate::db::sessions::find_session_by_token_hash(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if session.revoked_at.is_some() || session.expires_at <= OffsetDateTime::now_utc() {
        return Err(AppError::Unauthorized);
    }

    let user = crate::db::users::find_user_by_id(&state.db, session.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let access_token = token::issue_access_token(&user, &state.jwt_secret)?;
    let refresh_token = refresh_tokens::generate_refresh_token();
    let replacement_hash = refresh_tokens::hash_refresh_token(&refresh_token)?;
    let replacement_session = NewRefreshSession {
        user_id: user.id,
        token_hash: replacement_hash,
        expires_at: OffsetDateTime::now_utc() + Duration::days(30),
    };

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    crate::db::sessions::revoke_session_tx(&mut tx, session.id).await?;
    crate::db::sessions::create_session_tx(&mut tx, replacement_session).await?;

    tx.commit()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if let Err(error) =
        crate::db::redis::mark_refresh_token_revoked(&state.redis, &token_hash, session.expires_at)
            .await
    {
        tracing::warn!(?error, "failed to cache revoked refresh token");
    }
    crate::db::audit_logs::record_auth_event(&state.db, Some(user.id), "session.refreshed").await;

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
    if let Err(error) =
        crate::db::redis::mark_refresh_token_revoked(&state.redis, &token_hash, session.expires_at)
            .await
    {
        tracing::warn!(?error, "failed to cache revoked refresh token");
    }
    crate::db::audit_logs::record_auth_event(
        &state.db,
        Some(session.user_id),
        "session.logged_out",
    )
    .await;

    Ok(StatusCode::NO_CONTENT.into_response())
}
