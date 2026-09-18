use crate::config::{CorsConfig, OAuthConfig};
use crate::metrics::AppMetrics;
use redis::Client;
use reqwest::Client as HttpClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: Client,
    pub jwt_secret: String,
    pub trust_proxy_headers: bool,
    pub metrics_token: Option<String>,
    pub http_client: HttpClient,
    pub cors: CorsConfig,
    pub oauth: OAuthConfig,
    pub metrics: AppMetrics,
}
