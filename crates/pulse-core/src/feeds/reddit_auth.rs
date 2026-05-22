use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::error::FeedError;

const TOKEN_URL: &str = "https://www.reddit.com/api/v1/access_token";
const USER_AGENT: &str = "Pulse/0.1 (+https://github.com/avinthakur080/pulse-rs; feed-reader)";
/// Refresh 5 minutes before the 1-hour expiry to avoid mid-request expiry.
const TOKEN_LIFETIME: Duration = Duration::from_secs(3300);

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Holds Reddit OAuth2 script-app credentials and caches the current bearer token.
pub struct RedditAuth {
    pub client_id: String,
    pub client_secret: String,
    cached: Mutex<Option<(String, Instant)>>,
}

impl RedditAuth {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            cached: Mutex::new(None),
        }
    }

    /// Returns a valid bearer token, fetching a new one if the cached one has expired.
    pub async fn token(&self, http: &Client) -> Result<String, FeedError> {
        let mut guard = self.cached.lock().await;

        if let Some((ref token, fetched_at)) = *guard
            && fetched_at.elapsed() < TOKEN_LIFETIME
        {
            return Ok(token.clone());
        }

        let resp = http
            .post(TOKEN_URL)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .header("User-Agent", USER_AGENT)
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(|e| FeedError::Network {
                url: TOKEN_URL.to_string(),
                source: e,
            })?;

        let status = resp.status();
        if !status.is_success() {
            return Err(FeedError::Http {
                url: TOKEN_URL.to_string(),
                status: status.as_u16(),
                message: format!(
                    "Reddit OAuth token fetch failed: {}",
                    status.canonical_reason().unwrap_or("unknown")
                ),
            });
        }

        let tr: TokenResponse = resp.json().await.map_err(|e| FeedError::Network {
            url: TOKEN_URL.to_string(),
            source: e,
        })?;

        *guard = Some((tr.access_token.clone(), Instant::now()));
        Ok(tr.access_token)
    }

    /// Force-expire the cached token so the next call to `token()` fetches a fresh one.
    pub async fn invalidate(&self) {
        *self.cached.lock().await = None;
    }
}
