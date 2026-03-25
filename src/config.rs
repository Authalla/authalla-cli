use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Auth method determines how the CLI authenticates with the API.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// User token flow via `authalla login` (OAuth2 Authorization Code + PKCE)
    Login,
    /// M2M client credentials flow via `authalla config set`
    ClientCredentials,
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::ClientCredentials
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: i64,
}

/// Unified config supporting both login (user token) and client_credentials (M2M) flows.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Which auth flow to use. Defaults to client_credentials for backward compat.
    #[serde(default)]
    pub auth_method: AuthMethod,

    // --- Login (user token) fields ---
    /// The admin tenant's OIDC issuer URL (used for login auth method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    // --- Shared / M2M fields ---
    /// OAuth2 client ID (CLI client for login, M2M client for client_credentials)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    /// API base URL (used for client_credentials auth method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<CachedToken>,
}

impl Config {
    /// Create a new config for M2M client credentials flow.
    pub fn new_client_credentials(api_url: String, client_id: String, client_secret: String) -> Self {
        Config {
            auth_method: AuthMethod::ClientCredentials,
            issuer_url: None,
            access_token: None,
            refresh_token: None,
            id_token: None,
            expires_at: None,
            user: None,
            account_id: None,
            tenant_id: None,
            client_id: Some(client_id),
            api_url: Some(api_url),
            client_secret: Some(client_secret),
            token: None,
        }
    }

    /// Create a new config for user login flow.
    pub fn new_login(
        issuer_url: String,
        client_id: String,
        access_token: String,
        refresh_token: String,
        id_token: Option<String>,
        expires_at: i64,
        user: UserInfo,
    ) -> Self {
        Config {
            auth_method: AuthMethod::Login,
            issuer_url: Some(issuer_url),
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            id_token,
            expires_at: Some(expires_at),
            user: Some(user),
            account_id: None,
            tenant_id: None,
            client_id: Some(client_id),
            api_url: None,
            client_secret: None,
            token: None,
        }
    }

    /// Returns the base URL for API requests, depending on auth method.
    pub fn base_url(&self) -> Result<String> {
        match self.auth_method {
            AuthMethod::Login => {
                let url = self.issuer_url.as_ref()
                    .context("No issuer_url configured. Run `authalla login` first.")?;
                Ok(url.trim_end_matches('/').to_string())
            }
            AuthMethod::ClientCredentials => {
                let url = self.api_url.as_ref()
                    .context("No api_url configured. Run `authalla config set` first.")?;
                Ok(url.trim_end_matches('/').to_string())
            }
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("authalla");
    Ok(config_dir.join("config.json"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Could not read config at {}. Run `authalla login` or `authalla config set` first.", path.display()))?;
    let config: Config = serde_json::from_str(&contents)
        .context("Invalid config file format")?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(config)?;
    fs::write(&path, &contents)?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
