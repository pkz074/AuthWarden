use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{errors::AppError, models::user::NewUser, services::password, state::AppState};

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
