use reqwest::Client;
use serde::Deserialize;
use url::Url;

use crate::{config::OAuthProviderConfig, errors::AppError};

const GOOGLE_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

pub const GOOGLE_PROVIDER: &str = "google";

#[derive(Debug)]
pub struct GoogleProfile {
    pub provider_user_id: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfoResponse {
    sub: String,
    email: String,
    email_verified: bool,
}

struct GoogleApiEndpoints<'a> {
    token_url: &'a str,
    userinfo_url: &'a str,
}

impl Default for GoogleApiEndpoints<'static> {
    fn default() -> Self {
        Self {
            token_url: GOOGLE_TOKEN_URL,
            userinfo_url: GOOGLE_USERINFO_URL,
        }
    }
}

pub fn authorize_url(config: &OAuthProviderConfig, state: &str) -> Result<String, AppError> {
    let mut url = Url::parse(GOOGLE_AUTHORIZE_URL).map_err(|_| AppError::InternalServerError)?;

    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", state);

    Ok(url.to_string())
}

pub async fn exchange_code_for_access_token(
    client: &Client,
    config: &OAuthProviderConfig,
    code: &str,
) -> Result<String, AppError> {
    exchange_code_for_access_token_with_url(
        client,
        config,
        code,
        GoogleApiEndpoints::default().token_url,
    )
    .await
}

async fn exchange_code_for_access_token_with_url(
    client: &Client,
    config: &OAuthProviderConfig,
    code: &str,
    token_url: &str,
) -> Result<String, AppError> {
    let response = client
        .post(token_url)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", config.redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let token_response = response
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if token_response.access_token.is_empty() {
        return Err(AppError::Unauthorized);
    }

    Ok(token_response.access_token)
}

pub async fn fetch_profile(client: &Client, access_token: &str) -> Result<GoogleProfile, AppError> {
    fetch_profile_with_endpoints(client, access_token, GoogleApiEndpoints::default()).await
}

async fn fetch_profile_with_endpoints(
    client: &Client,
    access_token: &str,
    endpoints: GoogleApiEndpoints<'_>,
) -> Result<GoogleProfile, AppError> {
    let response = client
        .get(endpoints.userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let user_info = response
        .json::<GoogleUserInfoResponse>()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if !user_info.email_verified {
        return Err(AppError::Unauthorized);
    }

    Ok(GoogleProfile {
        provider_user_id: user_info.sub,
        email: user_info.email,
    })
}

#[cfg(test)]
mod tests {
    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn authorize_url_contains_required_google_parameters() {
        let config = OAuthProviderConfig {
            client_id: "client-id".to_string(),
            client_secret: "secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/google/callback".to_string(),
        };

        let url = authorize_url(&config, "state-token").unwrap();
        let url = Url::parse(&url).unwrap();

        assert_eq!(
            url.as_str().split('?').next().unwrap(),
            GOOGLE_AUTHORIZE_URL
        );
        assert_eq!(query_value(&url, "client_id").as_deref(), Some("client-id"));
        assert_eq!(query_value(&url, "response_type").as_deref(), Some("code"));
        assert_eq!(
            query_value(&url, "scope").as_deref(),
            Some("openid email profile")
        );
        assert_eq!(query_value(&url, "state").as_deref(), Some("state-token"));
    }

    fn query_value(url: &Url, key: &str) -> Option<String> {
        url.query_pairs()
            .find(|(query_key, _)| query_key == key)
            .map(|(_, value)| value.to_string())
    }

    #[tokio::test]
    async fn exchanges_code_for_access_token() {
        let server = spawn_server(Router::new().route(
            "/token",
            post(|| async { Json(json!({ "access_token": "google-token" })) }),
        ))
        .await;
        let config = test_config();
        let client = Client::new();

        let token = exchange_code_for_access_token_with_url(
            &client,
            &config,
            "google-code",
            &format!("{server}/token"),
        )
        .await
        .unwrap();

        assert_eq!(token, "google-token");
    }

    #[tokio::test]
    async fn rejects_failed_token_exchange() {
        let server = spawn_server(Router::new().route(
            "/token",
            post(|| async { (StatusCode::BAD_REQUEST, "bad code") }),
        ))
        .await;
        let config = test_config();
        let client = Client::new();

        let result = exchange_code_for_access_token_with_url(
            &client,
            &config,
            "google-code",
            &format!("{server}/token"),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_malformed_token_response() {
        let server = spawn_server(Router::new().route(
            "/token",
            post(|| async { Json(json!({ "not_access_token": "missing" })) }),
        ))
        .await;
        let config = test_config();
        let client = Client::new();

        let result = exchange_code_for_access_token_with_url(
            &client,
            &config,
            "google-code",
            &format!("{server}/token"),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fetches_verified_userinfo_profile() {
        let server = spawn_server(Router::new().route(
            "/userinfo",
            get(|| async {
                Json(json!({
                    "sub": "google-user-id",
                    "email": "google@example.com",
                    "email_verified": true
                }))
            }),
        ))
        .await;
        let client = Client::new();

        let profile = fetch_profile_with_endpoints(
            &client,
            "google-token",
            GoogleApiEndpoints {
                token_url: "",
                userinfo_url: &format!("{server}/userinfo"),
            },
        )
        .await
        .unwrap();

        assert_eq!(profile.provider_user_id, "google-user-id");
        assert_eq!(profile.email, "google@example.com");
    }

    #[tokio::test]
    async fn rejects_unverified_email() {
        let server = spawn_server(Router::new().route(
            "/userinfo",
            get(|| async {
                Json(json!({
                    "sub": "google-user-id",
                    "email": "google@example.com",
                    "email_verified": false
                }))
            }),
        ))
        .await;
        let client = Client::new();

        let result = fetch_profile_with_endpoints(
            &client,
            "google-token",
            GoogleApiEndpoints {
                token_url: "",
                userinfo_url: &format!("{server}/userinfo"),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_malformed_userinfo_response() {
        let server = spawn_server(Router::new().route(
            "/userinfo",
            get(|| async { Json(json!({ "sub": "missing-email" })) }),
        ))
        .await;
        let client = Client::new();

        let result = fetch_profile_with_endpoints(
            &client,
            "google-token",
            GoogleApiEndpoints {
                token_url: "",
                userinfo_url: &format!("{server}/userinfo"),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn provider_timeout_is_unauthorized() {
        let server = spawn_server(Router::new().route("/userinfo", get(slow_response))).await;
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();

        let result = fetch_profile_with_endpoints(
            &client,
            "google-token",
            GoogleApiEndpoints {
                token_url: "",
                userinfo_url: &format!("{server}/userinfo"),
            },
        )
        .await;

        assert!(result.is_err());
    }

    async fn spawn_server(router: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        format!("http://{addr}")
    }

    async fn slow_response() -> Json<serde_json::Value> {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Json(json!({
            "sub": "google-user-id",
            "email": "google@example.com",
            "email_verified": true
        }))
    }

    fn test_config() -> OAuthProviderConfig {
        OAuthProviderConfig {
            client_id: "client-id".to_string(),
            client_secret: "secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/google/callback".to_string(),
        }
    }
}
