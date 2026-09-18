use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

use crate::{
    db,
    errors::AppError,
    models::{
        session::{NewRefreshSession, TokenPair},
        user::User,
    },
    services::{
        github_oauth, google_oauth, oauth_login, oauth_state, refresh_token as refresh_tokens,
        token,
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn github_login(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let config = state
        .oauth
        .github
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("github oauth is not configured".to_string()))?;

    let oauth_state = oauth_state::generate_oauth_state();
    oauth_state::store_oauth_state(&state.redis, &oauth_state, github_oauth::GITHUB_PROVIDER)
        .await?;

    let url = github_oauth::authorize_url(config, &oauth_state)?;

    Ok(Redirect::to(&url))
}

pub async fn github_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, AppError> {
    let config = state
        .oauth
        .github
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("github oauth is not configured".to_string()))?;

    oauth_state::validate_oauth_state(&state.redis, &query.state, github_oauth::GITHUB_PROVIDER)
        .await?;

    let access_token =
        github_oauth::exchange_code_for_access_token(&state.http_client, config, &query.code)
            .await?;
    let profile = github_oauth::fetch_profile(&state.http_client, &access_token).await?;
    let user = oauth_login::find_or_create_oauth_user(
        &state.db,
        github_oauth::GITHUB_PROVIDER,
        profile.provider_user_id,
        profile.email,
    )
    .await?;
    let response = issue_session_tokens(&state, &user).await?;

    db::audit_logs::record_auth_event(&state.db, Some(user.id), "oauth.github_login").await;
    state.metrics.record_oauth_auth_success();

    Ok(Json(response).into_response())
}

pub async fn google_login(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let config = state
        .oauth
        .google
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("google oauth is not configured".to_string()))?;

    let oauth_state = oauth_state::generate_oauth_state();
    oauth_state::store_oauth_state(&state.redis, &oauth_state, google_oauth::GOOGLE_PROVIDER)
        .await?;

    let url = google_oauth::authorize_url(config, &oauth_state)?;

    Ok(Redirect::to(&url))
}

pub async fn google_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, AppError> {
    let config = state
        .oauth
        .google
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("google oauth is not configured".to_string()))?;

    oauth_state::validate_oauth_state(&state.redis, &query.state, google_oauth::GOOGLE_PROVIDER)
        .await?;

    let access_token =
        google_oauth::exchange_code_for_access_token(&state.http_client, config, &query.code)
            .await?;
    let profile = google_oauth::fetch_profile(&state.http_client, &access_token).await?;
    let user = oauth_login::find_or_create_oauth_user(
        &state.db,
        google_oauth::GOOGLE_PROVIDER,
        profile.provider_user_id,
        profile.email,
    )
    .await?;
    let response = issue_session_tokens(&state, &user).await?;

    db::audit_logs::record_auth_event(&state.db, Some(user.id), "oauth.google_login").await;
    state.metrics.record_oauth_auth_success();

    Ok(Json(response).into_response())
}

async fn issue_session_tokens(state: &AppState, user: &User) -> Result<TokenPair, AppError> {
    let access_token = token::issue_access_token(user, &state.jwt_secret)?;
    let refresh_token = refresh_tokens::generate_refresh_token();
    let token_hash = refresh_tokens::hash_refresh_token(&refresh_token)?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let new_session = NewRefreshSession {
        user_id: user.id,
        token_hash,
        expires_at,
    };

    db::sessions::create_session(&state.db, new_session).await?;

    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}
