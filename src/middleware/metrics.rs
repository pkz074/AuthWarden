use std::sync::Arc;

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};

use crate::state::AppState;

pub async fn record_http_metrics(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    state
        .metrics
        .record_http_request(&method, &path, response.status().as_u16());

    response
}
