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
    let response = client
        .post(GITHUB_TOKEN_URL)
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
    let user = client
        .get(GITHUB_USER_URL)
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
        .get(GITHUB_EMAILS_URL)
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
}
