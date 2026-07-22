use axum::{
    Json,
    response::{IntoResponse, Response},
};

use crate::{errors::AppError, extractors::auth_user::AuthenticatedUser, models::auth::MeResponse};

pub async fn me(user: AuthenticatedUser) -> Result<Response, AppError> {
    let response = MeResponse {
        id: user.user_id,
        email: user.email,
    };

    Ok(Json(response).into_response())
}
