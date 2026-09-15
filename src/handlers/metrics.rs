use std::sync::Arc;

use axum::{
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::IntoResponse,
};

use crate::state::AppState;

pub async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        state.metrics.render_prometheus(),
    )
}
