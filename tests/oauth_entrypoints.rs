mod support;

use authwarden::{build_app, services::oauth_state};
use axum::http::StatusCode;
use tower::ServiceExt;

use support::{
    app_state, get, github_oauth, google_oauth, no_oauth, response_text, test_db, test_redis,
};

#[tokio::test]
#[ignore = "requires Docker Postgres and Redis"]
async fn oauth_pages_redirects_state_and_bad_callbacks_work() {
    let db = test_db().await;
    let redis = test_redis();
    let state = app_state(db.clone(), redis.clone(), no_oauth());

    assert_auth_pages_render(state).await;
    assert_oauth_state_is_consumed(&redis).await;
    assert_oauth_login_redirect_stores_state(
        db.clone(),
        redis.clone(),
        github_oauth(),
        "/auth/github",
        "https://github.com/login/oauth/authorize",
        "github-client-id",
        "github",
    )
    .await;
    assert_oauth_callback_rejects_bad_state(
        db.clone(),
        redis.clone(),
        github_oauth(),
        "/auth/github/callback?code=fake-code&state=missing-state",
    )
    .await;
    assert_oauth_login_redirect_stores_state(
        db,
        redis,
        google_oauth(),
        "/auth/google",
        "https://accounts.google.com/o/oauth2/v2/auth",
        "google-client-id",
        "google",
    )
    .await;
}

async fn assert_auth_pages_render(state: std::sync::Arc<authwarden::state::AppState>) {
    let response = build_app(state.clone())
        .oneshot(get("/login"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Continue with GitHub"));
    assert!(body.contains("Continue with Google"));
    assert!(body.contains("/auth/github"));
    assert!(body.contains("/auth/google"));

    let response = build_app(state).oneshot(get("/register")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("Create your AuthWarden account"));
    assert!(body.contains("Continue with GitHub"));
    assert!(body.contains("Continue with Google"));
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
    db: sqlx::PgPool,
    redis: redis::Client,
    oauth: authwarden::config::OAuthConfig,
    path: &str,
    expected_authorize_url: &str,
    expected_client_id: &str,
    expected_provider: &str,
) {
    let state = app_state(db, redis.clone(), oauth);

    let response = build_app(state).oneshot(get(path)).await.unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );

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

async fn assert_oauth_callback_rejects_bad_state(
    db: sqlx::PgPool,
    redis: redis::Client,
    oauth: authwarden::config::OAuthConfig,
    path: &str,
) {
    let state = app_state(db, redis, oauth);

    let response = build_app(state).oneshot(get(path)).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );
}
