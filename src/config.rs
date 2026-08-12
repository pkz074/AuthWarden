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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn oauth_provider_is_disabled_when_any_required_value_is_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_oauth_env();

        set_env("GITHUB_CLIENT_ID", "github-client-id");
        set_env("GITHUB_CLIENT_SECRET", "github-client-secret");

        let config = OAuthConfig::from_env();

        assert!(config.github.is_none());
    }

    #[test]
    fn oauth_provider_is_disabled_when_required_value_is_blank() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_oauth_env();

        set_env("GOOGLE_CLIENT_ID", "google-client-id");
        set_env("GOOGLE_CLIENT_SECRET", "   ");
        set_env(
            "GOOGLE_REDIRECT_URI",
            "http://localhost:8080/auth/google/callback",
        );

        let config = OAuthConfig::from_env();

        assert!(config.google.is_none());
    }

    #[test]
    fn oauth_provider_is_loaded_when_all_values_are_present() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_oauth_env();

        set_env("GITHUB_CLIENT_ID", "github-client-id");
        set_env("GITHUB_CLIENT_SECRET", "github-client-secret");
        set_env(
            "GITHUB_REDIRECT_URI",
            "http://localhost:8080/auth/github/callback",
        );

        let config = OAuthConfig::from_env();
        let github = config.github.unwrap();

        assert_eq!(github.client_id, "github-client-id");
        assert_eq!(github.client_secret, "github-client-secret");
        assert_eq!(
            github.redirect_uri,
            "http://localhost:8080/auth/github/callback"
        );
    }

    #[test]
    fn app_config_uses_defaults_and_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_app_env();
        clear_oauth_env();

        set_env("APP_HOST", "0.0.0.0");
        set_env("APP_PORT", "9090");
        set_env("REDIS_URL", "redis://localhost:6380");

        let config = AppConfig::from_env();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9090);
        assert_eq!(config.redis_url, "redis://localhost:6380");
        assert_eq!(config.bind_addr(), "0.0.0.0:9090");
    }

    fn clear_oauth_env() {
        remove_env("GITHUB_CLIENT_ID");
        remove_env("GITHUB_CLIENT_SECRET");
        remove_env("GITHUB_REDIRECT_URI");
        remove_env("GOOGLE_CLIENT_ID");
        remove_env("GOOGLE_CLIENT_SECRET");
        remove_env("GOOGLE_REDIRECT_URI");
    }

    fn clear_app_env() {
        remove_env("APP_HOST");
        remove_env("APP_PORT");
        remove_env("REDIS_URL");
    }

    fn set_env(key: &str, value: &str) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env(key: &str) {
        unsafe {
            env::remove_var(key);
        }
    }
}
