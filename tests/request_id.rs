use std::sync::Arc;

use authwarden::{
    build_app,
    config::{CorsConfig, OAuthConfig},
    middleware::request_id::REQUEST_ID_HEADER,
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
const JWT_SECRET: &str = "authwarden-request-id-test-secret";

#[tokio::test]
async fn responses_include_generated_request_id() {
    let state = test_state();
    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = build_app(state).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let request_id = response.headers().get(&REQUEST_ID_HEADER).unwrap();
    assert_eq!(request_id.to_str().unwrap().len(), 36);
}

#[tokio::test]
async fn responses_preserve_incoming_request_id() {
    let state = test_state();
    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .header(&REQUEST_ID_HEADER, "client-request-id")
        .body(Body::empty())
        .unwrap();

    let response = build_app(state).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(&REQUEST_ID_HEADER).unwrap(),
        "client-request-id"
    );
}

#[tokio::test]
async fn error_responses_include_request_id() {
    let state = test_state();
    let request = Request::builder()
        .method("GET")
        .uri("/auth/github")
        .body(Body::empty())
        .unwrap();

    let response = build_app(state).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response.headers().contains_key(&REQUEST_ID_HEADER));
}

fn test_state() -> Arc<AppState> {
    let db = PgPoolOptions::new()
        .connect_lazy(DATABASE_URL)
        .expect("create lazy postgres pool");
    let redis = redis::Client::open(REDIS_URL).expect("create redis client");

    Arc::new(AppState {
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
    })
}
