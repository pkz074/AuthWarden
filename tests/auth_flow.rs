use std::sync::Arc;

use authwarden::{
    build_app,
    config::{OAuthConfig, OAuthProviderConfig},
    services::oauth_state,
    state::AppState,
};
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
async fn auth_flow_covers_core_auth_and_oauth_entrypoints() {
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
        oauth: OAuthConfig {
            github: None,
            google: None,
        },
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

    let oauth_only_email = format!("oauth-only-{}@authwarden.test", Uuid::new_v4());
    authwarden::db::users::create_oauth_user(&db, oauth_only_email.clone())
        .await
        .unwrap();
    let oauth_only_login = post_form(
        "/login",
        &[("email", &oauth_only_email), ("password", password)],
    );
    let response = build_app(state.clone())
        .oneshot(oauth_only_login)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

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
    assert_oauth_state_is_consumed(&redis).await;
    assert_auth_pages_render(state.clone()).await;
    assert_oauth_login_redirect_stores_state(
        db.clone(),
        redis.clone(),
        OAuthConfig {
            github: Some(OAuthProviderConfig {
                client_id: "github-client-id".to_string(),
                client_secret: "github-client-secret".to_string(),
                redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
            }),
            google: None,
        },
        "/auth/github",
        "https://github.com/login/oauth/authorize",
        "github-client-id",
        "github",
    )
    .await;
    assert_oauth_login_redirect_stores_state(
        db,
        redis,
        OAuthConfig {
            github: None,
            google: Some(OAuthProviderConfig {
                client_id: "google-client-id".to_string(),
                client_secret: "google-client-secret".to_string(),
                redirect_uri: "http://localhost:8080/auth/google/callback".to_string(),
            }),
        },
        "/auth/google",
        "https://accounts.google.com/o/oauth2/v2/auth",
        "google-client-id",
        "google",
    )
    .await;
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

async fn response_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

async fn assert_auth_pages_render(state: Arc<AppState>) {
    let login = Request::builder()
        .method("GET")
        .uri("/login")
        .body(Body::empty())
        .unwrap();
    let response = build_app(state.clone()).oneshot(login).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Continue with GitHub"));
    assert!(body.contains("Continue with Google"));
    assert!(body.contains("/auth/github"));
    assert!(body.contains("/auth/google"));

    let register = Request::builder()
        .method("GET")
        .uri("/register")
        .body(Body::empty())
        .unwrap();
    let response = build_app(state).oneshot(register).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Create your AuthWarden account"));
    assert!(body.contains("Continue with GitHub"));
    assert!(body.contains("Continue with Google"));
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

async fn assert_oauth_state_is_consumed(redis: &redis::Client) {
    let state = oauth_state::generate_oauth_state();

    oauth_state::store_oauth_state(redis, &state, "github")
        .await
        .unwrap();
    oauth_state::validate_oauth_state(redis, &state, "github")
        .await
        .unwrap();

    let replay = oauth_state::validate_oauth_state(redis, &state, "github").await;
    assert!(replay.is_err());

    let wrong_provider_state = oauth_state::generate_oauth_state();
    oauth_state::store_oauth_state(redis, &wrong_provider_state, "github")
        .await
        .unwrap();

    let wrong_provider =
        oauth_state::validate_oauth_state(redis, &wrong_provider_state, "google").await;
    assert!(wrong_provider.is_err());

    let missing_state =
        oauth_state::validate_oauth_state(redis, &oauth_state::generate_oauth_state(), "github")
            .await;
    assert!(missing_state.is_err());
}

async fn assert_oauth_login_redirect_stores_state(
    db: PgPool,
    redis: redis::Client,
    oauth: OAuthConfig,
    path: &str,
    expected_authorize_url: &str,
    expected_client_id: &str,
    expected_provider: &str,
) {
    let state = Arc::new(AppState {
        db,
        redis: redis.clone(),
        jwt_secret: JWT_SECRET.to_string(),
        oauth,
    });

    let request = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = build_app(state).oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response
        .headers()
        .get("location")
        .expect("redirect location header")
        .to_str()
        .unwrap();
    let location = url::Url::parse(location).unwrap();
    assert_eq!(
        location.as_str().split('?').next().unwrap(),
        expected_authorize_url
    );
    assert_eq!(
        location
            .query_pairs()
            .find(|(key, _)| key == "client_id")
            .map(|(_, value)| value.to_string())
            .as_deref(),
        Some(expected_client_id)
    );

    let state_value = location
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .expect("oauth state query parameter");

    oauth_state::validate_oauth_state(&redis, &state_value, expected_provider)
        .await
        .unwrap();
}
