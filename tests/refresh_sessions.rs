mod support;

use authwarden::build_app;
use axum::http::StatusCode;
use redis::AsyncCommands;
use sqlx::PgPool;
use tower::ServiceExt;

use support::{
    app_state, no_oauth, post_form, post_raw, register_and_login, response_json, test_db,
    test_redis, unique_email,
};

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn refresh_rotation_logout_replay_and_audit_logs_work() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db.clone(), redis.clone(), no_oauth());
    let email = unique_email("refresh");
    let password = "Password123";
    let (_, original_refresh_token) = register_and_login(state.clone(), &email, password).await;

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/refresh",
            &[("refresh_token", &original_refresh_token)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let refresh_body = response_json(response).await;
    let rotated_refresh_token = refresh_body["refresh_token"].as_str().unwrap();
    assert_ne!(rotated_refresh_token, original_refresh_token);

    let response = build_app(state.clone())
        .oneshot(post_raw("/refresh", ""))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/refresh",
            &[("refresh_token", &original_refresh_token)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/logout",
            &[("refresh_token", rotated_refresh_token)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = build_app(state.clone())
        .oneshot(post_raw("/logout", ""))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = build_app(state)
        .oneshot(post_form(
            "/refresh",
            &[("refresh_token", rotated_refresh_token)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    assert_audit_events(&db, &email).await;
    assert_revoked_token_keys_exist(&redis).await;
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
