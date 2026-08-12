mod support;

use authwarden::{build_app, db};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use support::{
    app_state, get, get_with_bearer, no_oauth, post_form, post_raw, response_json, test_db,
    test_redis, unique_email,
};

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn password_auth_flow_covers_validation_login_and_me() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db.clone(), redis, no_oauth());
    let email = unique_email("integration");
    let password = "Password123";

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/register",
            &[("email", "bad-email"), ("password", password)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = build_app(state.clone())
        .oneshot(post_raw("/register", "email=missing-password"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/register",
            &[("email", &email), ("password", password)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/register",
            &[("email", &email), ("password", password)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let oauth_only_email = unique_email("oauth-only");
    db::users::create_oauth_user(&db, oauth_only_email.clone())
        .await
        .unwrap();
    let response = build_app(state.clone())
        .oneshot(post_form(
            "/login",
            &[("email", &oauth_only_email), ("password", password)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = build_app(state.clone())
        .oneshot(post_form(
            "/login",
            &[("email", &email), ("password", password)],
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let login_body = response_json(response).await;
    let access_token = login_body["access_token"].as_str().unwrap();

    let response = build_app(state.clone()).oneshot(get("/me")).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = build_app(state.clone())
        .oneshot(get_with_bearer("/me", "Token invalid"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = build_app(state.clone())
        .oneshot(get_with_bearer("/me", "Bearer invalid.jwt.token"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let me = Request::builder()
        .method("GET")
        .uri("/me")
        .header("Authorization", format!("Bearer {access_token}"))
        .body(Body::empty())
        .unwrap();
    let response = build_app(state).oneshot(me).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let me_body = response_json(response).await;
    assert_eq!(me_body["email"], email);
}
