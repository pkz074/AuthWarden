use std::env;

pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
    pub oauth: OAuthConfig,
}

#[derive(Clone, Debug)]
pub struct OAuthConfig {
    pub github: Option<OAuthProviderConfig>,
    pub google: Option<OAuthProviderConfig>,
}

#[derive(Clone, Debug)]
pub struct OAuthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port: u16 = std::env::var("APP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("APP_PORT must be a valid number");

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let oauth = OAuthConfig::from_env();

        Self {
            host,
            port,
            redis_url,
            oauth,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl OAuthConfig {
    fn from_env() -> Self {
        Self {
            github: OAuthProviderConfig::from_env(
                "GITHUB_CLIENT_ID",
                "GITHUB_CLIENT_SECRET",
                "GITHUB_REDIRECT_URI",
            ),
            google: OAuthProviderConfig::from_env(
                "GOOGLE_CLIENT_ID",
                "GOOGLE_CLIENT_SECRET",
                "GOOGLE_REDIRECT_URI",
            ),
        }
    }
}

impl OAuthProviderConfig {
    fn from_env(
        client_id_key: &str,
        client_secret_key: &str,
        redirect_uri_key: &str,
    ) -> Option<Self> {
        let client_id = env_var_non_empty(client_id_key)?;
        let client_secret = env_var_non_empty(client_secret_key)?;
        let redirect_uri = env_var_non_empty(redirect_uri_key)?;

        Some(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }
}

fn env_var_non_empty(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}
