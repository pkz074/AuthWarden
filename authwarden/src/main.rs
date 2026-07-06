use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

mod config;
mod db;
mod errors;
mod extractors;
mod handlers;
mod models;
mod services;
mod state;

use config::AppConfig;
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env();
    tracing::info!("starting authwarden on {}", config.bind_addr());

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://authwarden:authwarden@localhost:5432/authwarden".to_string()
    });

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to db");

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let state = Arc::new(AppState { db, jwt_secret });

    let app = Router::new()
        .route("/", get(handlers::pages::login_page))
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/logout", post(handlers::sessions::logout))
        .route("/refresh", post(handlers::sessions::refresh))
        .route("/me", get(handlers::account::me))
        .route("/health", get(handlers::health::health))
        .route("/health/db", get(handlers::health::health_db))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("failed to bind address");

    println!("listening on http://{}", config.bind_addr());

    axum::serve(listener, app).await.expect("server failed");
}
