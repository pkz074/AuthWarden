use reqwest::Client;
use serde::Deserialize;
use url::Url;

use crate::{config::OAuthProviderConfig, errors::AppError};

const GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_EMAILS_URL: &str = "https://api.github.com/user/emails";
const GITHUB_USER_AGENT: &str = "authwarden";

pub const GITHUB_PROVIDER: &str = "github";

#[derive(Debug)]
pub struct GitHubProfile {
    pub provider_user_id: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUserResponse {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubEmailResponse {
    email: String,
    primary: bool,
    verified: bool,
}

struct GitHubApiEndpoints<'a> {
    token_url: &'a str,
    user_url: &'a str,
    emails_url: &'a str,
}

impl Default for GitHubApiEndpoints<'static> {
    fn default() -> Self {
        Self {
            token_url: GITHUB_TOKEN_URL,
            user_url: GITHUB_USER_URL,
            emails_url: GITHUB_EMAILS_URL,
        }
    }
}

pub fn authorize_url(config: &OAuthProviderConfig, state: &str) -> Result<String, AppError> {
    let mut url = Url::parse(GITHUB_AUTHORIZE_URL).map_err(|_| AppError::InternalServerError)?;

    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("scope", "user:email")
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
        GitHubApiEndpoints::default().token_url,
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
        .header("Accept", "application/json")
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", config.redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let token_response = response
        .json::<GitHubTokenResponse>()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if token_response.access_token.is_empty() {
        return Err(AppError::Unauthorized);
    }

    Ok(token_response.access_token)
}

pub async fn fetch_profile(client: &Client, access_token: &str) -> Result<GitHubProfile, AppError> {
    fetch_profile_with_endpoints(client, access_token, GitHubApiEndpoints::default()).await
}

async fn fetch_profile_with_endpoints(
    client: &Client,
    access_token: &str,
    endpoints: GitHubApiEndpoints<'_>,
) -> Result<GitHubProfile, AppError> {
    let user = client
        .get(endpoints.user_url)
        .bearer_auth(access_token)
        .header("User-Agent", GITHUB_USER_AGENT)
        .send()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if !user.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let user = user
        .json::<GitHubUserResponse>()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let emails = client
        .get(endpoints.emails_url)
        .bearer_auth(access_token)
        .header("User-Agent", GITHUB_USER_AGENT)
        .send()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    if !emails.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let emails = emails
        .json::<Vec<GitHubEmailResponse>>()
        .await
        .map_err(|_| AppError::Unauthorized)?;

    let email = emails
        .into_iter()
        .find(|email| email.primary && email.verified)
        .map(|email| email.email)
        .ok_or(AppError::Unauthorized)?;

    Ok(GitHubProfile {
        provider_user_id: user.id.to_string(),
        email,
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
    fn authorize_url_contains_required_github_parameters() {
        let config = OAuthProviderConfig {
            client_id: "client-id".to_string(),
            client_secret: "secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
        };

        let url = authorize_url(&config, "state-token").unwrap();

        assert!(url.starts_with(GITHUB_AUTHORIZE_URL));
        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("scope=user%3Aemail"));
        assert!(url.contains("state=state-token"));
    }

    #[tokio::test]
    async fn exchanges_code_for_access_token() {
        let server = spawn_server(Router::new().route(
            "/token",
            post(|| async { Json(json!({ "access_token": "github-token" })) }),
        ))
        .await;
        let config = test_config();
        let client = Client::new();

        let token = exchange_code_for_access_token_with_url(
            &client,
            &config,
            "github-code",
            &format!("{server}/token"),
        )
        .await
        .unwrap();

        assert_eq!(token, "github-token");
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
            "github-code",
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
            "github-code",
            &format!("{server}/token"),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn fetches_verified_primary_profile_email() {
        let server = spawn_server(
            Router::new()
                .route("/user", get(|| async { Json(json!({ "id": 42 })) }))
                .route(
                    "/emails",
                    get(|| async {
                        Json(json!([
                            { "email": "secondary@example.com", "primary": false, "verified": true },
                            { "email": "primary@example.com", "primary": true, "verified": true }
                        ]))
                    }),
                ),
        )
        .await;
        let client = Client::new();

        let profile = fetch_profile_with_endpoints(
            &client,
            "github-token",
            GitHubApiEndpoints {
                token_url: "",
                user_url: &format!("{server}/user"),
                emails_url: &format!("{server}/emails"),
            },
        )
        .await
        .unwrap();

        assert_eq!(profile.provider_user_id, "42");
        assert_eq!(profile.email, "primary@example.com");
    }

    #[tokio::test]
    async fn rejects_profile_without_verified_primary_email() {
        let server = spawn_server(
            Router::new()
                .route("/user", get(|| async { Json(json!({ "id": 42 })) }))
                .route(
                    "/emails",
                    get(|| async {
                        Json(json!([
                            { "email": "primary@example.com", "primary": true, "verified": false }
                        ]))
                    }),
                ),
        )
        .await;
        let client = Client::new();

        let result = fetch_profile_with_endpoints(
            &client,
            "github-token",
            GitHubApiEndpoints {
                token_url: "",
                user_url: &format!("{server}/user"),
                emails_url: &format!("{server}/emails"),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_malformed_profile_response() {
        let server = spawn_server(
            Router::new().route("/user", get(|| async { Json(json!({ "not_id": 42 })) })),
        )
        .await;
        let client = Client::new();

        let result = fetch_profile_with_endpoints(
            &client,
            "github-token",
            GitHubApiEndpoints {
                token_url: "",
                user_url: &format!("{server}/user"),
                emails_url: &format!("{server}/emails"),
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn provider_timeout_is_unauthorized() {
        let server = spawn_server(Router::new().route("/user", get(slow_response))).await;
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();

        let result = fetch_profile_with_endpoints(
            &client,
            "github-token",
            GitHubApiEndpoints {
                token_url: "",
                user_url: &format!("{server}/user"),
                emails_url: &format!("{server}/emails"),
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
        Json(json!({ "id": 42 }))
    }

    fn test_config() -> OAuthProviderConfig {
        OAuthProviderConfig {
            client_id: "client-id".to_string(),
            client_secret: "secret".to_string(),
            redirect_uri: "http://localhost:8080/auth/github/callback".to_string(),
        }
    }
}
