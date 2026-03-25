use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;
use crate::config;

#[derive(Subcommand)]
pub enum AccountCommands {
    /// List all accounts the current user has access to.
    List,
    /// Switch the active account.
    Select {
        /// Account ID to switch to
        id: String,
    },
}

pub fn run(cmd: AccountCommands) -> Result<()> {
    match cmd {
        AccountCommands::List => list(),
        AccountCommands::Select { id } => select(&id),
    }
}

fn list() -> Result<()> {
    let cfg = config::load()?;
    let api = ApiClient::new_without_tenant()?;
    let me = api.get("/api/v1/me")?;

    let accounts = me["accounts"]
        .as_array()
        .context("Expected accounts array")?;

    let active_account = cfg.account_id.as_deref().unwrap_or("");

    println!(
        "  {:<20} {:<20} {}",
        "NAME", "ID", "ROLE"
    );
    for account in accounts {
        let name = account["name"].as_str().unwrap_or("");
        let id = account["id"].as_str().unwrap_or("");
        let role = account["role"].as_str().unwrap_or("");
        let marker = if id == active_account { "*" } else { " " };
        println!("{} {:<20} {:<20} {}", marker, name, id, role);
    }

    Ok(())
}

fn select(account_id: &str) -> Result<()> {
    let mut cfg = config::load()?;
    let api = ApiClient::new_without_tenant()?;
    let me = api.get("/api/v1/me")?;

    let accounts = me["accounts"]
        .as_array()
        .context("Expected accounts array")?;

    let account = accounts
        .iter()
        .find(|a| a["id"].as_str() == Some(account_id))
        .with_context(|| format!("Account '{}' not found", account_id))?;

    let account_name = account["name"].as_str().unwrap_or("");
    cfg.account_id = Some(account_id.to_string());

    // Auto-select first tenant of the new account
    if let Some(tenants) = account["tenants"].as_array() {
        if let Some(tenant) = tenants.first() {
            let tenant_id = tenant["id"].as_str().unwrap_or_default().to_string();
            let tenant_name = tenant["name"].as_str().unwrap_or("default");
            cfg.tenant_id = Some(tenant_id.clone());
            eprintln!("Active account: {}", account_name);
            eprintln!("Active tenant: {} ({})", tenant_name, tenant_id);
        }
    }

    config::save(&cfg)?;
    Ok(())
}
