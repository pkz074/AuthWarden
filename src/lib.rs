use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use std::sync::Arc;

pub mod config;
pub mod db;
pub mod errors;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod state;

use state::AppState;

pub fn build_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(handlers::pages::login_page))
        .route(
            "/register",
            get(handlers::pages::register_page).post(handlers::auth::register),
        )
        .route(
            "/login",
            get(handlers::pages::login_page).post(handlers::auth::login),
        )
        .route("/auth/github", get(handlers::oauth::github_login))
        .route(
            "/auth/github/callback",
            get(handlers::oauth::github_callback),
        )
        .route("/auth/google", get(handlers::oauth::google_login))
        .route(
            "/auth/google/callback",
            get(handlers::oauth::google_callback),
        )
        .route("/logout", post(handlers::sessions::logout))
        .route("/refresh", post(handlers::sessions::refresh))
        .route("/me", get(handlers::account::me))
        .route("/health", get(handlers::health::health))
        .route("/health/db", get(handlers::health::health_db))
        .layer(axum_middleware::from_fn(
            middleware::security_headers::set_security_headers,
        ))
        .with_state(state)
}
