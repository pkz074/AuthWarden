use authwarden::{build_app, config::AppConfig, state::AppState};
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

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

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let redis =
        redis::Client::open(config.redis_url.clone()).expect("failed to create redis client");
    let http_client = Client::builder()
        .timeout(Duration::from_secs(config.oauth_http_timeout_seconds))
        .build()
        .expect("failed to create http client");

    let state = Arc::new(AppState {
        db,
        redis,
        jwt_secret,
        trust_proxy_headers: config.trust_proxy_headers,
        metrics_token: config.metrics_token.clone(),
        http_client,
        cors: config.cors.clone(),
        oauth: config.oauth.clone(),
        metrics: Default::default(),
    });

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("failed to bind address");

    println!("listening on http://{}", config.bind_addr());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("server failed");
}
