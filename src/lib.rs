use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

pub mod config;
pub mod db;
pub mod errors;
pub mod extractors;
pub mod handlers;
pub mod models;
pub mod services;
pub mod state;

use state::AppState;

pub fn build_app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(handlers::pages::login_page))
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/logout", post(handlers::sessions::logout))
        .route("/refresh", post(handlers::sessions::refresh))
        .route("/me", get(handlers::account::me))
        .route("/health", get(handlers::health::health))
        .route("/health/db", get(handlers::health::health_db))
        .with_state(state)
}
