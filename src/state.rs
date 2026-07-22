use redis::Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: Client,
    pub jwt_secret: String,
}
