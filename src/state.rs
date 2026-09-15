use crate::config::{CorsConfig, OAuthConfig};
use crate::metrics::AppMetrics;
use redis::Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: Client,
    pub jwt_secret: String,
    pub cors: CorsConfig,
    pub oauth: OAuthConfig,
    pub metrics: AppMetrics,
}
