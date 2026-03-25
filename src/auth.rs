use anyhow::{Context, Result};
use chrono::Utc;

use crate::config::{self, AuthMethod, CachedToken, Config};

/// Returns a valid access token, refreshing if needed.
pub fn get_token(cfg: &mut Config) -> Result<String> {
    match cfg.auth_method {
        AuthMethod::Login => get_user_token(cfg),
        AuthMethod::ClientCredentials => get_m2m_token(cfg),
    }
}

/// Discover the token endpoint from the issuer's OIDC configuration.
fn discover_token_endpoint(issuer_url: &str) -> Result<String> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer_url.trim_end_matches('/')
    );

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("Failed to fetch OIDC configuration from {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("OIDC discovery failed ({})", resp.status());
    }

    let config: serde_json::Value = resp.json().context("Invalid OIDC configuration")?;

    config["token_endpoint"]
        .as_str()
        .context("Missing token_endpoint in OIDC configuration")
        .map(|s| s.to_string())
}

/// User token flow: use stored access_token, refresh via standard OAuth2 refresh_token grant.
fn get_user_token(cfg: &mut Config) -> Result<String> {
    // Check if we have a valid access token (with 30s buffer)
    if let (Some(ref token), Some(expires_at)) = (&cfg.access_token, cfg.expires_at) {
        if expires_at > Utc::now().timestamp() + 30 {
            return Ok(token.clone());
        }
    }

    let refresh_token = cfg
        .refresh_token
        .as_ref()
        .context("No refresh token found. Run `authalla login` to authenticate.")?;

    let issuer_url = cfg
        .issuer_url
        .as_ref()
        .context("No issuer_url configured. Run `authalla login` first.")?;

    let client_id = cfg
        .client_id
        .as_ref()
        .context("No client_id configured. Run `authalla login` first.")?;

    let token_endpoint = discover_token_endpoint(issuer_url)?;

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&token_endpoint)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id.as_str()),
        ])
        .send()
        .context("Failed to reach token endpoint")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        anyhow::bail!(
            "Token refresh failed ({}): {}. Run `authalla login` to re-authenticate.",
            status,
            body
        );
    }

    let body: serde_json::Value = resp.json().context("Invalid token response")?;

    let access_token = body["access_token"]
        .as_str()
        .context("Missing access_token in response")?
        .to_string();

    let expires_in = body["expires_in"].as_i64().unwrap_or(900);

    cfg.access_token = Some(access_token.clone());
    cfg.expires_at = Some(Utc::now().timestamp() + expires_in);

    // Server may rotate the refresh token
    if let Some(new_refresh) = body["refresh_token"].as_str() {
        cfg.refresh_token = Some(new_refresh.to_string());
    }

    // Update id_token if returned
    if let Some(new_id_token) = body["id_token"].as_str() {
        cfg.id_token = Some(new_id_token.to_string());
    }

    config::save(cfg)?;
    Ok(access_token)
}

/// M2M client credentials flow (existing behavior).
fn get_m2m_token(cfg: &mut Config) -> Result<String> {
    // Check if we have a cached token that's still valid (with 30s buffer)
    if let Some(ref token) = cfg.token {
        if token.expires_at > Utc::now().timestamp() + 30 {
            return Ok(token.access_token.clone());
        }
    }

    let api_url = cfg
        .api_url
        .as_ref()
        .context("No api_url configured. Run `authalla config set` first.")?;

    let client_id = cfg
        .client_id
        .as_ref()
        .context("No client_id configured. Run `authalla config set` first.")?;
    let client_secret = cfg
        .client_secret
        .as_ref()
        .context("No client_secret configured. Run `authalla config set` first.")?;

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/oauth2/token", api_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
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

    let expires_in = body["expires_in"].as_i64().unwrap_or(900);

    let token = CachedToken {
        access_token,
        expires_at: Utc::now().timestamp() + expires_in,
    };
    cfg.token = Some(token.clone());
    config::save(cfg)?;
    Ok(token.access_token)
}
