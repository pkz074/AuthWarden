use std::sync::Arc;

use authwarden::{
    build_app,
    config::{CorsConfig, OAuthConfig},
    middleware::security_headers::CONTENT_SECURITY_POLICY,
    state::AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

const DATABASE_URL: &str = "postgres://authwarden:authwarden@localhost:5432/authwarden";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const JWT_SECRET: &str = "authwarden-security-header-test-secret";

#[tokio::test]
async fn responses_include_security_headers() {
    let db = PgPoolOptions::new()
        .connect_lazy(DATABASE_URL)
        .expect("create lazy postgres pool");
    let redis = redis::Client::open(REDIS_URL).expect("create redis client");
    let state = Arc::new(AppState {
        db,
        redis,
        jwt_secret: JWT_SECRET.to_string(),
        cors: CorsConfig {
            allowed_origins: vec![],
        },
        oauth: OAuthConfig {
            github: None,
            google: None,
        },
        metrics: Default::default(),
    });

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let response = build_app(state).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );
    assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
    assert_eq!(
        response.headers().get("referrer-policy").unwrap(),
        "no-referrer"
    );
    assert_eq!(
        response.headers().get("content-security-policy").unwrap(),
        CONTENT_SECURITY_POLICY
    );
}

#[tokio::test]
async fn error_responses_include_security_headers() {
    let db = PgPoolOptions::new()
        .connect_lazy(DATABASE_URL)
        .expect("create lazy postgres pool");
    let redis = redis::Client::open(REDIS_URL).expect("create redis client");
    let state = Arc::new(AppState {
        db,
        redis,
        jwt_secret: JWT_SECRET.to_string(),
        cors: CorsConfig {
            allowed_origins: vec![],
        },
        oauth: OAuthConfig {
            github: None,
            google: None,
        },
        metrics: Default::default(),
    });

    let request = Request::builder()
        .method("GET")
        .uri("/auth/github")
        .body(Body::empty())
        .unwrap();
    let response = build_app(state).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_security_headers(&response);
}

fn assert_security_headers(response: &axum::response::Response) {
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );
    assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
    assert_eq!(
        response.headers().get("referrer-policy").unwrap(),
        "no-referrer"
    );
    assert_eq!(
        response.headers().get("content-security-policy").unwrap(),
        CONTENT_SECURITY_POLICY
    );
}
