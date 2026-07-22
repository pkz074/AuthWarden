use std::sync::Arc;

use authwarden::{build_app, state::AppState};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use redis::AsyncCommands;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

const DATABASE_URL: &str = "postgres://authwarden:authwarden@localhost:5432/authwarden";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const JWT_SECRET: &str = "authwarden-integration-test-secret";

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn auth_flow_covers_phase_1_and_phase_2() {
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .expect("connect to test postgres");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("run migrations");

    let redis = redis::Client::open(REDIS_URL).expect("create redis client");
    let state = Arc::new(AppState {
        db: db.clone(),
        redis: redis.clone(),
        jwt_secret: JWT_SECRET.to_string(),
    });

    let email = format!("integration-{}@authwarden.test", Uuid::new_v4());
    let password = "Password123";

    let bad_email = post_form(
        "/register",
        &[("email", "bad-email"), ("password", password)],
    );
    let response = build_app(state.clone()).oneshot(bad_email).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let register = post_form("/register", &[("email", &email), ("password", password)]);
    let response = build_app(state.clone()).oneshot(register).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let duplicate = post_form("/register", &[("email", &email), ("password", password)]);
    let response = build_app(state.clone()).oneshot(duplicate).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let login = post_form("/login", &[("email", &email), ("password", password)]);
    let response = build_app(state.clone()).oneshot(login).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let login_body = response_json(response).await;
    let access_token = login_body["access_token"].as_str().unwrap();
    let original_refresh_token = login_body["refresh_token"].as_str().unwrap();

    let me = Request::builder()
        .method("GET")
        .uri("/me")
        .header("Authorization", format!("Bearer {access_token}"))
        .body(Body::empty())
        .unwrap();
    let response = build_app(state.clone()).oneshot(me).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let me_body = response_json(response).await;
    assert_eq!(me_body["email"], email);

    let refresh = post_form("/refresh", &[("refresh_token", original_refresh_token)]);
    let response = build_app(state.clone()).oneshot(refresh).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let refresh_body = response_json(response).await;
    let rotated_refresh_token = refresh_body["refresh_token"].as_str().unwrap();
    assert_ne!(rotated_refresh_token, original_refresh_token);

    let replay_old_token = post_form("/refresh", &[("refresh_token", original_refresh_token)]);
    let response = build_app(state.clone())
        .oneshot(replay_old_token)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let logout = post_form("/logout", &[("refresh_token", rotated_refresh_token)]);
    let response = build_app(state.clone()).oneshot(logout).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let replay_logged_out_token =
        post_form("/refresh", &[("refresh_token", rotated_refresh_token)]);
    let response = build_app(state.clone())
        .oneshot(replay_logged_out_token)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    assert_audit_events(&db, &email).await;
    assert_revoked_token_keys_exist(&redis).await;
}

fn post_form(path: &str, fields: &[(&str, &str)]) -> Request<Body> {
    let body = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    Request::builder()
        .method("POST")
        .uri(path)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn assert_audit_events(db: &PgPool, email: &str) {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT event_type
        FROM audit_logs
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        ORDER BY event_type
        "#,
    )
    .bind(email)
    .fetch_all(db)
    .await
    .unwrap();

    assert_eq!(
        rows,
        vec![
            "session.logged_out".to_string(),
            "session.refreshed".to_string(),
            "user.logged_in".to_string(),
            "user.registered".to_string(),
        ]
    );
}

async fn assert_revoked_token_keys_exist(redis: &redis::Client) {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .expect("connect to test redis");

    let keys: Vec<String> = connection
        .keys("revoked_refresh_token:*")
        .await
        .expect("read redis revoked-token keys");

    assert!(keys.len() >= 2);
}
