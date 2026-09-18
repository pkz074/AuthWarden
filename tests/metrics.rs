use std::sync::Arc;

use authwarden::{
    build_app,
    config::{CorsConfig, OAuthConfig},
    metrics::AppMetrics,
    state::AppState,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

const DATABASE_URL: &str = "postgres://authwarden:authwarden@localhost:5432/authwarden";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const JWT_SECRET: &str = "authwarden-metrics-test-secret";

#[tokio::test]
async fn metrics_endpoint_reports_http_request_totals() {
    let state = test_state(AppMetrics::default());

    let response = build_app(state.clone())
        .oneshot(get("/health"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = build_app(state).oneshot(get("/metrics")).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "text/plain; version=0.0.4; charset=utf-8"
    );
    let body = response_text(response).await;
    assert!(body.contains(
        "authwarden_http_requests_total{method=\"GET\",path=\"/health\",status=\"200\"} 1"
    ));
}

#[tokio::test]
async fn metrics_endpoint_reports_auth_session_counters() {
    let metrics = AppMetrics::default();
    metrics.record_password_auth_success();
    metrics.record_password_auth_failure();
    metrics.record_oauth_auth_success();
    metrics.record_oauth_auth_failure();
    metrics.record_refresh_rotation();
    metrics.record_logout();

    let response = build_app(test_state(metrics))
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    let body = response_text(response).await;

    assert!(body.contains("authwarden_auth_success_total{flow=\"password\"} 1"));
    assert!(body.contains("authwarden_auth_failure_total{flow=\"password\"} 1"));
    assert!(body.contains("authwarden_auth_success_total{flow=\"oauth\"} 1"));
    assert!(body.contains("authwarden_auth_failure_total{flow=\"oauth\"} 1"));
    assert!(body.contains("authwarden_refresh_rotations_total 1"));
    assert!(body.contains("authwarden_logouts_total 1"));
}

#[tokio::test]
async fn metrics_endpoint_requires_bearer_token_when_configured() {
    let state = test_state_with_token(AppMetrics::default(), Some("metrics-secret".to_string()));

    let missing = build_app(state.clone())
        .oneshot(get("/metrics"))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let wrong = build_app(state.clone())
        .oneshot(get_with_bearer("/metrics", "wrong-token"))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);

    let allowed = build_app(state)
        .oneshot(get_with_bearer("/metrics", "metrics-secret"))
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
}

fn test_state(metrics: AppMetrics) -> Arc<AppState> {
    test_state_with_token(metrics, None)
}

fn test_state_with_token(metrics: AppMetrics, metrics_token: Option<String>) -> Arc<AppState> {
    let db = PgPoolOptions::new()
        .connect_lazy(DATABASE_URL)
        .expect("create lazy postgres pool");
    let redis = redis::Client::open(REDIS_URL).expect("create redis client");

    Arc::new(AppState {
        db,
        redis,
        jwt_secret: JWT_SECRET.to_string(),
        trust_proxy_headers: false,
        metrics_token,
        http_client: reqwest::Client::new(),
        cors: CorsConfig {
            allowed_origins: vec![],
        },
        oauth: OAuthConfig {
            github: None,
            google: None,
        },
        metrics,
    })
}

fn get_with_bearer(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header("Authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

async fn response_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}
