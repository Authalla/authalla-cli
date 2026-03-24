mod api;
mod auth;
mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "authalla", about = "CLI for the Authalla OAuth2 API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure API credentials
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Manage tenants
    Tenant {
        #[command(subcommand)]
        command: commands::tenant::TenantCommands,
    },
    /// Manage users
    User {
        #[command(subcommand)]
        command: commands::user::UserCommands,
    },
    /// Manage theme settings
    Theme {
        #[command(subcommand)]
        command: commands::theme::ThemeCommands,
    },
    /// Manage custom domains
    #[command(name = "custom-domain")]
    CustomDomain {
        #[command(subcommand)]
        command: commands::custom_domain::CustomDomainCommands,
    },
    /// Manage custom email domains
    #[command(name = "custom-email")]
    CustomEmail {
        #[command(subcommand)]
        command: commands::custom_email::CustomEmailCommands,
    },
    /// Manage OAuth2 clients
    Client {
        #[command(subcommand)]
        command: commands::client::ClientCommands,
    },
    /// Manage social login providers
    #[command(name = "social-login")]
    SocialLogin {
        #[command(subcommand)]
        command: commands::social_login::SocialLoginCommands,
    },
    /// Fetch well-known endpoints (OpenID configuration, JWKS)
    #[command(name = "well-known")]
    WellKnown {
        #[command(subcommand)]
        command: commands::well_known::WellKnownCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set API credentials
    Set {
        /// Authalla API base URL
        #[arg(long)]
        api_url: String,
        /// OAuth2 client ID
        #[arg(long)]
        client_id: String,
        /// OAuth2 client secret
        #[arg(long)]
        client_secret: String,
    },
    /// Show current configuration
    Show,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::Set {
                api_url,
                client_id,
                client_secret,
            } => {
                let cfg = config::Config {
                    api_url,
                    client_id,
                    client_secret,
                    token: None,
                };
                config::save(&cfg)?;
                eprintln!("Configuration saved.");
                Ok(())
            }
            ConfigCommands::Show => {
                let cfg = config::load()?;
                // Redact the secret for display
                let display = serde_json::json!({
                    "api_url": cfg.api_url,
                    "client_id": cfg.client_id,
                    "client_secret": format!("{}…", &cfg.client_secret[..8.min(cfg.client_secret.len())]),
                    "token": cfg.token.as_ref().map(|t| serde_json::json!({
                        "expires_at": t.expires_at,
                        "has_token": true,
                    })),
                });
                println!("{}", serde_json::to_string_pretty(&display)?);
                Ok(())
            }
        },
        Commands::Tenant { command } => commands::tenant::run(command),
        Commands::User { command } => commands::user::run(command),
        Commands::Theme { command } => commands::theme::run(command),
        Commands::CustomDomain { command } => commands::custom_domain::run(command),
        Commands::CustomEmail { command } => commands::custom_email::run(command),
        Commands::Client { command } => commands::client::run(command),
        Commands::SocialLogin { command } => commands::social_login::run(command),
        Commands::WellKnown { command } => commands::well_known::run(command),
    }
}
