use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

use crate::{
    db,
    errors::AppError,
    models::{
        oauth_account::NewOAuthAccount,
        session::{NewRefreshSession, TokenPair},
        user::User,
    },
    services::{github_oauth, google_oauth, oauth_state, refresh_token as refresh_tokens, token},
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

    let client = Client::new();
    let access_token =
        github_oauth::exchange_code_for_access_token(&client, config, &query.code).await?;
    let profile = github_oauth::fetch_profile(&client, &access_token).await?;
    let user = find_or_create_oauth_user(
        &state,
        github_oauth::GITHUB_PROVIDER,
        profile.provider_user_id,
        profile.email,
    )
    .await?;
    let response = issue_session_tokens(&state, &user).await?;

    db::audit_logs::record_auth_event(&state.db, Some(user.id), "oauth.github_login").await;

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

    let client = Client::new();
    let access_token =
        google_oauth::exchange_code_for_access_token(&client, config, &query.code).await?;
    let profile = google_oauth::fetch_profile(&client, &access_token).await?;
    let user = find_or_create_oauth_user(
        &state,
        google_oauth::GOOGLE_PROVIDER,
        profile.provider_user_id,
        profile.email,
    )
    .await?;
    let response = issue_session_tokens(&state, &user).await?;

    db::audit_logs::record_auth_event(&state.db, Some(user.id), "oauth.google_login").await;

    Ok(Json(response).into_response())
}

async fn find_or_create_oauth_user(
    state: &AppState,
    provider: &str,
    provider_user_id: String,
    provider_email: String,
) -> Result<User, AppError> {
    if let Some(account) = db::oauth_accounts::find_oauth_account_by_provider_id(
        &state.db,
        provider,
        &provider_user_id,
    )
    .await?
    {
        db::oauth_accounts::update_oauth_account_email(
            &state.db,
            account.id,
            Some(normalize_email(&provider_email)),
        )
        .await?;

        return db::users::find_user_by_id(&state.db, account.user_id)
            .await?
            .ok_or(AppError::Unauthorized);
    }

    let email = normalize_email(&provider_email);
    let user = match db::users::find_user_by_email(&state.db, &email).await? {
        Some(user) => user,
        None => db::users::create_oauth_user(&state.db, email.clone()).await?,
    };

    let new_account = NewOAuthAccount {
        user_id: user.id,
        provider: provider.to_string(),
        provider_user_id,
        provider_email: Some(email),
    };

    db::oauth_accounts::create_oauth_account(&state.db, new_account).await?;

    Ok(user)
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

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
