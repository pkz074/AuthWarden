use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("resource not found")]
    NotFound,

    #[error("invalid email or password")]
    Unauthorized,

    #[error("internal server error")]
    InternalServerError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = self.to_string();
        (status, body).into_response()
    }
}
