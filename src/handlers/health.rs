use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use std::sync::Arc;

use crate::{errors::AppError, state::AppState};

pub async fn health() -> Html<&'static str> {
    Html("<h1> Hello, world!</h1><p>This is a healthy response.</p>")
}

pub async fn health_db(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok((StatusCode::OK, "database ok"))
}
