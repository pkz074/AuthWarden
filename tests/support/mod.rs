#![allow(dead_code)]

use std::sync::Arc;

use authwarden::{
    build_app,
    config::{CorsConfig, OAuthConfig, OAuthProviderConfig},
    state::AppState,
};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

pub const DATABASE_URL: &str = "postgres://authwarden:authwarden@localhost:5432/authwarden";
pub const REDIS_URL: &str = "redis://127.0.0.1:6379";
pub const JWT_SECRET: &str = "authwarden-integration-test-secret";

pub async fn test_db() -> PgPool {
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .expect("connect to test postgres");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("run migrations");

    db
}

pub fn test_redis() -> redis::Client {
    redis::Client::open(REDIS_URL).expect("create redis client")
}

pub fn app_state(db: PgPool, redis: redis::Client, oauth: OAuthConfig) -> Arc<AppState> {
    Arc::new(AppState {
        db,
        redis,
        jwt_secret: JWT_SECRET.to_string(),
        cors: CorsConfig {
            allowed_origins: vec![],
        },
        oauth,
    })
}

pub fn no_oauth() -> OAuthConfig {
    OAuthConfig {
        github: None,
        google: None,
    }
}

pub fn github_oauth() -> OAuthConfig {
    OAuthConfig {
        github: Some(OAuthProviderConfig {
            client_id: "github-client-id".to_string(),
            client_secret: "github-client-secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
        }),
        google: None,
    }
}

pub fn google_oauth() -> OAuthConfig {
    OAuthConfig {
        github: None,
        google: Some(OAuthProviderConfig {
            client_id: "google-client-id".to_string(),
            client_secret: "google-client-secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/google/callback".to_string(),
        }),
    }
}

pub fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@authwarden.test", Uuid::new_v4())
}

pub fn post_form(path: &str, fields: &[(&str, &str)]) -> Request<Body> {
    let body = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    post_raw(path, &body)
}

pub fn post_raw(path: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

pub fn get_with_bearer(path: &str, authorization: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header("Authorization", authorization)
        .body(Body::empty())
        .unwrap()
}

pub async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

pub async fn response_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

pub async fn register_and_login(
    state: Arc<AppState>,
    email: &str,
    password: &str,
) -> (String, String) {
    let register = post_form("/register", &[("email", email), ("password", password)]);
    let response = build_app(state.clone()).oneshot(register).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::CREATED);

    let login = post_form("/login", &[("email", email), ("password", password)]);
    let response = build_app(state).oneshot(login).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response_json(response).await;
    (
        body["access_token"].as_str().unwrap().to_string(),
        body["refresh_token"].as_str().unwrap().to_string(),
    )
}

use tower::ServiceExt;
