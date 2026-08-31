mod support;

use authwarden::{build_app, middleware::rate_limit::RATE_LIMIT_MAX_REQUESTS};
use axum::http::StatusCode;
use tower::ServiceExt;
use uuid::Uuid;

use support::{app_state, get, no_oauth, post_raw, test_db, test_redis};

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn sensitive_routes_return_too_many_requests_after_limit() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db, redis, no_oauth());
    let client_id = format!("203.0.113.{}", Uuid::new_v4());

    for _ in 0..RATE_LIMIT_MAX_REQUESTS {
        let response = build_app(state.clone())
            .oneshot(rate_limited_login_request(&client_id))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let response = build_app(state)
        .oneshot(rate_limited_login_request(&client_id))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn non_sensitive_routes_are_not_rate_limited() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db, redis, no_oauth());

    for _ in 0..(RATE_LIMIT_MAX_REQUESTS + 1) {
        let response = build_app(state.clone())
            .oneshot(get("/health"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

fn rate_limited_login_request(client_id: &str) -> axum::http::Request<axum::body::Body> {
    let mut request = post_raw("/login", "email=bad-email&password=Password123");
    request
        .headers_mut()
        .insert("x-forwarded-for", client_id.parse().unwrap());
    request
}
