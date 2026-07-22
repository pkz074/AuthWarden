use authwarden::{build_app, config::AppConfig, state::AppState};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

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

    let redis =
        redis::Client::open(config.redis_url.clone()).expect("failed to create redis client");

    let state = Arc::new(AppState {
        db,
        redis,
        jwt_secret,
    });

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("failed to bind address");

    println!("listening on http://{}", config.bind_addr());

    axum::serve(listener, app).await.expect("server failed");
}
