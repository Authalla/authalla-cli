use anyhow::Result;

use crate::config;

pub fn run() -> Result<()> {
    let mut cfg = config::load()?;

    // Clear login tokens
    cfg.access_token = None;
    cfg.refresh_token = None;
    cfg.id_token = None;
    cfg.expires_at = None;
    cfg.user = None;
    cfg.account_id = None;
    cfg.tenant_id = None;

    // Reset auth method to client_credentials if M2M creds exist
    if cfg.client_secret.is_some() && cfg.api_url.is_some() {
        cfg.auth_method = config::AuthMethod::ClientCredentials;
    }

    config::save(&cfg)?;
    eprintln!("Logged out successfully.");
    Ok(())
}
