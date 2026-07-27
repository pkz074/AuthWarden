use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

use crate::errors::AppError;

pub async fn login_page() -> Result<impl IntoResponse, AppError> {
    let content = read_login_template().await?;
    Ok((StatusCode::OK, Html(content)))
}

pub async fn register_page() -> Result<impl IntoResponse, AppError> {
    let content = read_register_template().await?;
    Ok((StatusCode::OK, Html(content)))
}

async fn read_login_template() -> Result<String, AppError> {
    tokio::fs::read_to_string("templates/login.html")
        .await
        .map_err(|_| AppError::InternalServerError)
}

async fn read_register_template() -> Result<String, AppError> {
    tokio::fs::read_to_string("templates/register.html")
        .await
        .map_err(|_| AppError::InternalServerError)
}
