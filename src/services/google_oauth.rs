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
    let response = client
        .post(GOOGLE_TOKEN_URL)
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
    let response = client
        .get(GOOGLE_USERINFO_URL)
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
}
