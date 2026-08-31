use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request},
    middleware::Next,
    response::Response,
};
use redis::{AsyncCommands, Client};

use crate::{errors::AppError, state::AppState};

pub const RATE_LIMIT_MAX_REQUESTS: u64 = 10;
pub const RATE_LIMIT_WINDOW_SECONDS: i64 = 60;

pub async fn rate_limit_sensitive_routes(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    if !is_rate_limited_route(&method, &path) {
        return Ok(next.run(request).await);
    }

    let client_id = client_identifier(request.headers());

    match allow_request(&state.redis, &method, &path, &client_id).await {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => Err(AppError::TooManyRequests),
        Err(error) => {
            tracing::warn!(?error, "failed to apply redis rate limit");
            Ok(next.run(request).await)
        }
    }
}

async fn allow_request(
    redis: &Client,
    method: &Method,
    path: &str,
    client_id: &str,
) -> Result<bool, AppError> {
    let key = rate_limit_key(method, path, client_id);
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let count: u64 = connection
        .incr(&key, 1_u8)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if count == 1 {
        let _: bool = connection
            .expire(&key, RATE_LIMIT_WINDOW_SECONDS)
            .await
            .map_err(|_| AppError::InternalServerError)?;
    }

    Ok(count <= RATE_LIMIT_MAX_REQUESTS)
}

fn is_rate_limited_route(method: &Method, path: &str) -> bool {
    matches!(
        (method, path),
        (&Method::POST, "/login")
            | (&Method::POST, "/register")
            | (&Method::POST, "/refresh")
            | (&Method::GET, "/auth/github/callback")
            | (&Method::GET, "/auth/google/callback")
    )
}

fn client_identifier(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("unknown")
        .to_string()
}

fn rate_limit_key(method: &Method, path: &str, client_id: &str) -> String {
    let path = path.trim_start_matches('/').replace('/', ":");
    let client_id = client_id.replace([':', ' '], "_");

    format!("rate_limit:{method}:{path}:{client_id}")
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn detects_sensitive_routes() {
        assert!(is_rate_limited_route(&Method::POST, "/login"));
        assert!(is_rate_limited_route(&Method::POST, "/register"));
        assert!(is_rate_limited_route(&Method::POST, "/refresh"));
        assert!(is_rate_limited_route(&Method::GET, "/auth/github/callback"));
        assert!(!is_rate_limited_route(&Method::GET, "/login"));
        assert!(!is_rate_limited_route(&Method::GET, "/health"));
    }

    #[test]
    fn extracts_first_forwarded_client_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 10.0.0.1"),
        );

        assert_eq!(client_identifier(&headers), "203.0.113.10");
    }

    #[test]
    fn falls_back_to_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("203.0.113.20"));

        assert_eq!(client_identifier(&headers), "203.0.113.20");
    }
}
