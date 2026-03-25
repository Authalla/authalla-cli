use anyhow::{Context, Result};
use clap::Subcommand;
use reqwest::blocking::Client;

use crate::config;

#[derive(Subcommand)]
pub enum WellKnownCommands {
    /// Fetch the OpenID Connect discovery document
    #[command(name = "openid-configuration")]
    OpenidConfiguration,
    /// Fetch the JSON Web Key Set (JWKS)
    Jwks,
}

pub fn run(cmd: WellKnownCommands) -> Result<()> {
    let cfg = config::load()?;
    let base_url = cfg.base_url()?;
    let client = Client::new();

    match cmd {
        WellKnownCommands::OpenidConfiguration => {
            let url = format!("{}/.well-known/openid-configuration", base_url);
            let resp = client
                .get(&url)
                .send()
                .with_context(|| format!("Request failed: GET {}", url))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                anyhow::bail!("API error ({}): {}", status, body);
            }

            let body: serde_json::Value = resp.json().context("Invalid JSON response")?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        WellKnownCommands::Jwks => {
            // First fetch the OpenID configuration to get the jwks_uri
            let discovery_url = format!("{}/.well-known/openid-configuration", base_url);
            let resp = client
                .get(&discovery_url)
                .send()
                .with_context(|| format!("Request failed: GET {}", discovery_url))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                anyhow::bail!("API error ({}): {}", status, body);
            }

            let discovery: serde_json::Value =
                resp.json().context("Invalid JSON response from discovery endpoint")?;

            let jwks_uri = discovery["jwks_uri"]
                .as_str()
                .context("No jwks_uri found in OpenID configuration")?;

            let resp = client
                .get(jwks_uri)
                .send()
                .with_context(|| format!("Request failed: GET {}", jwks_uri))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                anyhow::bail!("API error ({}): {}", status, body);
            }

            let body: serde_json::Value = resp.json().context("Invalid JSON response")?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
    }

    Ok(())
}
