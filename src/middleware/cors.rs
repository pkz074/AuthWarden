use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{
        HeaderMap, HeaderValue, Method, Request, StatusCode,
        header::{
            ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
            ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE, ACCESS_CONTROL_REQUEST_METHOD,
            ORIGIN, VARY,
        },
    },
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

const ALLOWED_METHODS: &str = "GET,POST,OPTIONS";
const ALLOWED_HEADERS: &str = "authorization,content-type,x-request-id";
const MAX_AGE_SECONDS: &str = "600";

pub async fn apply_cors_policy(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let origin = request
        .headers()
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let is_preflight = request.method() == Method::OPTIONS
        && request
            .headers()
            .contains_key(ACCESS_CONTROL_REQUEST_METHOD);

    let origin_is_allowed = origin
        .as_deref()
        .is_some_and(|origin| state.cors.allows_origin(origin));

    if is_preflight {
        if origin_is_allowed {
            let mut response = StatusCode::NO_CONTENT.into_response();
            add_cors_headers(response.headers_mut(), origin.as_deref().unwrap());
            return response;
        }

        return StatusCode::FORBIDDEN.into_response();
    }

    let mut response = next.run(request).await;

    if let Some(origin) = origin
        .as_deref()
        .filter(|origin| state.cors.allows_origin(origin))
    {
        add_cors_headers(response.headers_mut(), origin);
    }

    response
}

fn add_cors_headers(headers: &mut HeaderMap, origin: &str) {
    if let Ok(origin) = HeaderValue::from_str(origin) {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }

    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static(ALLOWED_METHODS),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static(ALLOWED_HEADERS),
    );
    headers.insert(
        ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static(MAX_AGE_SECONDS),
    );
    headers.insert(VARY, HeaderValue::from_static("Origin"));
}

#[cfg(test)]
mod tests {
    use crate::config::CorsConfig;

    #[test]
    fn cors_config_matches_exact_origins() {
        let cors = CorsConfig {
            allowed_origins: vec!["https://app.example.com".to_string()],
        };

        assert!(cors.allows_origin("https://app.example.com"));
        assert!(!cors.allows_origin("https://admin.example.com"));
        assert!(!cors.allows_origin("http://app.example.com"));
    }
}
