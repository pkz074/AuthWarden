use axum::{
    Json,
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

use crate::{
    errors::AppError,
    models::{
        auth::LoginForm,
        session::{NewRefreshSession, TokenPair},
        user::NewUser,
    },
    services::{password, refresh_token as refresh_tokens, token},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub email: String,
    pub password: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RegisterForm>,
) -> Result<impl IntoResponse, AppError> {
    validate_register_form(&form)?;
    let email = normalize_email(&form.email);

    let password_hash = password::hash_password(&form.password)?;

    let new_user = NewUser {
        email,
        password_hash,
    };

    let user = crate::db::users::create_user(&state.db, new_user).await?;
    crate::db::audit_logs::record_auth_event(&state.db, Some(user.id), "user.registered").await;

    Ok((StatusCode::CREATED, "user registered"))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    validate_login_form(&form)?;
    let email = normalize_email(&form.email);

    let user = crate::db::users::find_user_by_email(&state.db, &email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let password_is_valid = password::verify_password(&form.password, &user.password_hash)?;

    if !password_is_valid {
        return Err(AppError::Unauthorized);
    }

    let access_token = token::issue_access_token(&user, &state.jwt_secret)?;
    let refresh_token = refresh_tokens::generate_refresh_token();
    let token_hash = refresh_tokens::hash_refresh_token(&refresh_token)?;
    let expires_at = OffsetDateTime::now_utc() + Duration::days(30);

    let new_session = NewRefreshSession {
        user_id: user.id,
        token_hash,
        expires_at,
    };

    crate::db::sessions::create_session(&state.db, new_session).await?;
    crate::db::audit_logs::record_auth_event(&state.db, Some(user.id), "user.logged_in").await;

    let response = TokenPair {
        access_token,
        refresh_token,
    };

    Ok(Json(response).into_response())
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn validate_register_form(form: &RegisterForm) -> Result<(), AppError> {
    validate_email(&form.email)?;
    validate_password(&form.password)?;

    Ok(())
}

fn validate_login_form(form: &LoginForm) -> Result<(), AppError> {
    validate_email(&form.email)?;

    if form.password.is_empty() {
        return Err(AppError::BadRequest("password is required".to_string()));
    }

    Ok(())
}

fn validate_email(email: &str) -> Result<(), AppError> {
    let email = email.trim();

    if email.is_empty() {
        return Err(AppError::BadRequest("email is required".to_string()));
    }

    if email.len() > 254 {
        return Err(AppError::BadRequest("email is too long".to_string()));
    }

    if email.chars().any(char::is_whitespace) {
        return Err(AppError::BadRequest("must be a valid email".to_string()));
    }

    let Some((local_part, domain)) = email.split_once('@') else {
        return Err(AppError::BadRequest("must be a valid email".to_string()));
    };

    if local_part.is_empty()
        || domain.is_empty()
        || domain.contains('@')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
    {
        return Err(AppError::BadRequest("must be a valid email".to_string()));
    }

    Ok(())
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    if password.len() > 128 {
        return Err(AppError::BadRequest("password is too long".to_string()));
    }

    if password.trim().is_empty() {
        return Err(AppError::BadRequest("password is required".to_string()));
    }

    Ok(())
}
