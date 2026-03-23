use anyhow::{Context, Result};
use chrono::Utc;

use crate::config::{self, CachedToken, Config};

/// Returns a valid access token, refreshing if needed.
pub fn get_token(cfg: &mut Config) -> Result<String> {
    // Check if we have a cached token that's still valid (with 30s buffer)
    if let Some(ref token) = cfg.token {
        if token.expires_at > Utc::now().timestamp() + 30 {
            return Ok(token.access_token.clone());
        }
    }

    // Fetch a new token
    let token = fetch_token(cfg)?;
    cfg.token = Some(token.clone());
    config::save(cfg)?;
    Ok(token.access_token)
}

fn fetch_token(cfg: &Config) -> Result<CachedToken> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/oauth2/token", cfg.api_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", &cfg.client_id),
            ("client_secret", &cfg.client_secret),
        ])
        .send()
        .context("Failed to reach token endpoint")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("Token request failed ({}): {}", status, body);
    }

    let body: serde_json::Value = resp.json().context("Invalid token response")?;

    let access_token = body["access_token"]
        .as_str()
        .context("Missing access_token in response")?
        .to_string();

    let expires_in = body["expires_in"]
        .as_i64()
        .unwrap_or(900);

    Ok(CachedToken {
        access_token,
        expires_at: Utc::now().timestamp() + expires_in,
    })
}
