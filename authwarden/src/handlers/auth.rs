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

    let password_hash = password::hash_password(&form.password)?;

    let new_user = NewUser {
        email: form.email.trim().to_lowercase(),
        password_hash,
    };

    crate::db::users::create_user(&state.db, new_user).await?;

    Ok((StatusCode::CREATED, "user registered"))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let email = form.email.trim().to_lowercase();

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

    let response = TokenPair {
        access_token,
        refresh_token,
    };

    Ok(Json(response).into_response())
}

fn validate_register_form(form: &RegisterForm) -> Result<(), AppError> {
    if form.email.trim().is_empty() {
        return Err(AppError::BadRequest("email is required".to_string()));
    }

    if !form.email.trim().contains('@') {
        return Err(AppError::BadRequest("must be a valid email".to_string()));
    }

    if form.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    Ok(())
}
