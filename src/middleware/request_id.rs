use std::time::Instant;

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

pub async fn add_request_id_and_log(request: Request<Body>, next: Next) -> Response {
    let started_at = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let request_id = request_id_header(&request).unwrap_or_else(generate_request_id);

    let mut response = next.run(request).await;
    let status = response.status();
    let latency_ms = started_at.elapsed().as_millis();

    response
        .headers_mut()
        .insert(REQUEST_ID_HEADER, request_id.clone());

    tracing::info!(
        request_id = %request_id.to_str().unwrap_or("invalid-request-id"),
        method = %method,
        path = %path,
        status = status.as_u16(),
        latency_ms,
        "http request completed"
    );

    response
}

fn request_id_header(request: &Request<Body>) -> Option<HeaderValue> {
    request.headers().get(&REQUEST_ID_HEADER).cloned()
}

fn generate_request_id() -> HeaderValue {
    HeaderValue::from_str(&Uuid::new_v4().to_string()).expect("uuid is a valid header value")
}
