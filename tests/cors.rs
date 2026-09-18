use std::sync::Arc;

use authwarden::{
    build_app,
    config::{CorsConfig, OAuthConfig},
    state::AppState,
};
use axum::{
    body::Body,
    http::{
        Request, StatusCode,
        header::{
            ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
            ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE, ACCESS_CONTROL_REQUEST_METHOD,
            ORIGIN, VARY,
        },
    },
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

const DATABASE_URL: &str = "postgres://authwarden:authwarden@localhost:5432/authwarden";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const JWT_SECRET: &str = "authwarden-cors-test-secret";
const ALLOWED_ORIGIN: &str = "https://app.example.com";
const BLOCKED_ORIGIN: &str = "https://blocked.example.com";

#[tokio::test]
async fn default_cors_policy_does_not_allow_cross_origin_requests() {
    let response = build_app(test_state(vec![]))
        .oneshot(get_with_origin("/health", ALLOWED_ORIGIN))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[tokio::test]
async fn allowed_origin_gets_cors_headers() {
    let response = build_app(test_state(vec![ALLOWED_ORIGIN.to_string()]))
        .oneshot(get_with_origin("/health", ALLOWED_ORIGIN))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
        ALLOWED_ORIGIN
    );
    assert_eq!(
        response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_METHODS)
            .unwrap(),
        "GET,POST,OPTIONS"
    );
    assert_eq!(response.headers().get(VARY).unwrap(), "Origin");
}

#[tokio::test]
async fn allowed_preflight_returns_no_content() {
    let response = build_app(test_state(vec![ALLOWED_ORIGIN.to_string()]))
        .oneshot(preflight(ALLOWED_ORIGIN))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
        ALLOWED_ORIGIN
    );
    assert_eq!(
        response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_HEADERS)
            .unwrap(),
        "authorization,content-type,x-request-id"
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_MAX_AGE).unwrap(),
        "600"
    );
}

#[tokio::test]
async fn disallowed_preflight_is_forbidden() {
    let response = build_app(test_state(vec![ALLOWED_ORIGIN.to_string()]))
        .oneshot(preflight(BLOCKED_ORIGIN))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
}

fn test_state(allowed_origins: Vec<String>) -> Arc<AppState> {
    let db = PgPoolOptions::new()
        .connect_lazy(DATABASE_URL)
        .expect("create lazy postgres pool");
    let redis = redis::Client::open(REDIS_URL).expect("create redis client");

    Arc::new(AppState {
        db,
        redis,
        jwt_secret: JWT_SECRET.to_string(),
        trust_proxy_headers: false,
        metrics_token: None,
        http_client: reqwest::Client::new(),
        cors: CorsConfig { allowed_origins },
        oauth: OAuthConfig {
            github: None,
            google: None,
        },
        metrics: Default::default(),
    })
}

fn get_with_origin(path: &str, origin: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header(ORIGIN, origin)
        .body(Body::empty())
        .unwrap()
}

fn preflight(origin: &str) -> Request<Body> {
    Request::builder()
        .method("OPTIONS")
        .uri("/login")
        .header(ORIGIN, origin)
        .header(ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .unwrap()
}
