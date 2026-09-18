mod support;

use authwarden::{
    build_app,
    services::login_lockout::{LOGIN_LOCKOUT_MAX_FAILURES, LOGIN_LOCKOUT_SECONDS},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use redis::AsyncCommands;
use tower::ServiceExt;

use support::{
    app_state, no_oauth, post_form, register_and_login, test_db, test_redis, unique_email,
};

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn password_login_locks_after_repeated_failures() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db, redis.clone(), no_oauth());
    let email = unique_email("lockout");
    let password = "Password123";
    let client_id = unique_client_id();

    register_and_login(state.clone(), &email, password).await;

    for _ in 0..LOGIN_LOCKOUT_MAX_FAILURES {
        let response = build_app(state.clone())
            .oneshot(login_request(&email, "wrong-password", &client_id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = build_app(state)
        .oneshot(login_request(&email, password, &client_id))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_lockout_key_exists(&redis, &email, &client_id).await;
}

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn password_login_lockout_is_scoped_to_client_identity() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db, redis, no_oauth());
    let email = unique_email("lockout-client");
    let password = "Password123";
    let attacker_client_id = unique_client_id();
    let real_client_id = unique_client_id();

    register_and_login(state.clone(), &email, password).await;

    for _ in 0..LOGIN_LOCKOUT_MAX_FAILURES {
        let response = build_app(state.clone())
            .oneshot(login_request(&email, "wrong-password", &attacker_client_id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let attacker_response = build_app(state.clone())
        .oneshot(login_request(&email, password, &attacker_client_id))
        .await
        .unwrap();
    assert_eq!(attacker_response.status(), StatusCode::TOO_MANY_REQUESTS);

    let real_response = build_app(state)
        .oneshot(login_request(&email, password, &real_client_id))
        .await
        .unwrap();
    assert_eq!(real_response.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn successful_password_login_clears_previous_failures() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db, redis.clone(), no_oauth());
    let email = unique_email("lockout-clear");
    let password = "Password123";
    let client_id = unique_client_id();

    register_and_login(state.clone(), &email, password).await;

    for _ in 0..(LOGIN_LOCKOUT_MAX_FAILURES - 1) {
        let response = build_app(state.clone())
            .oneshot(login_request(&email, "wrong-password", &client_id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = build_app(state.clone())
        .oneshot(login_request(&email, password, &client_id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    for _ in 0..(LOGIN_LOCKOUT_MAX_FAILURES - 1) {
        let response = build_app(state.clone())
            .oneshot(login_request(&email, "wrong-password", &client_id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = build_app(state)
        .oneshot(login_request(&email, password, &client_id))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

fn login_request(email: &str, password: &str, client_id: &str) -> Request<Body> {
    let mut request = post_form("/login", &[("email", email), ("password", password)]);
    request
        .headers_mut()
        .insert("x-forwarded-for", client_id.parse().unwrap());
    request
}

fn unique_client_id() -> String {
    format!("203.0.113.{}", uuid::Uuid::new_v4())
}

async fn assert_lockout_key_exists(redis: &redis::Client, email: &str, client_id: &str) {
    let mut connection = redis
        .get_multiplexed_async_connection()
        .await
        .expect("connect to test redis");

    let exists: bool = connection
        .exists(format!("login_lockout:{email}:{client_id}"))
        .await
        .expect("read lockout key");
    let ttl: i64 = connection
        .ttl(format!("login_lockout:{email}:{client_id}"))
        .await
        .expect("read lockout ttl");

    assert!(exists);
    assert!(ttl > 0);
    assert!(ttl <= LOGIN_LOCKOUT_SECONDS as i64);
}
