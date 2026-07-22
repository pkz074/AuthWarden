use std::env;

pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        // TODO
        let port: u16 = std::env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("APP_PORT must be a valid number");

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        Self {
            host,
            port,
            redis_url,
        }
    }

    pub fn bind_addr(&self) -> String {
        // TODO
        format!("{}:{}", self.host, self.port)
    }
}
