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
    /// Authenticate via browser (OAuth2 Authorization Code + PKCE)
    Login {
        /// OIDC issuer URL of the admin tenant (default: https://authalla.com)
        #[arg(long)]
        issuer_url: Option<String>,
        /// OAuth2 client ID for the CLI (default: authalla-cli)
        #[arg(long)]
        client_id: Option<String>,
    },
    /// Clear stored authentication tokens
    Logout,
    /// Manage accounts
    Accounts {
        #[command(subcommand)]
        command: commands::account::AccountCommands,
    },
    /// Configure M2M API credentials
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
    /// Set M2M API credentials (for CI/CD and scripts)
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
        Commands::Login {
            issuer_url,
            client_id,
        } => commands::login::run(issuer_url, client_id),
        Commands::Logout => commands::logout::run(),
        Commands::Accounts { command } => commands::account::run(command),
        Commands::Config { command } => match command {
            ConfigCommands::Set {
                api_url,
                client_id,
                client_secret,
            } => {
                let cfg = config::Config::new_client_credentials(api_url, client_id, client_secret);
                config::save(&cfg)?;
                eprintln!("Configuration saved.");
                Ok(())
            }
            ConfigCommands::Show => {
                let cfg = config::load()?;
                let display = match cfg.auth_method {
                    config::AuthMethod::Login => {
                        serde_json::json!({
                            "auth_method": "login",
                            "issuer_url": cfg.issuer_url,
                            "client_id": cfg.client_id,
                            "user": cfg.user.as_ref().map(|u| serde_json::json!({
                                "email": u.email,
                                "name": u.name,
                            })),
                            "account_id": cfg.account_id,
                            "tenant_id": cfg.tenant_id,
                            "has_token": cfg.access_token.is_some(),
                            "expires_at": cfg.expires_at,
                        })
                    }
                    config::AuthMethod::ClientCredentials => {
                        let client_id = cfg.client_id.as_deref().unwrap_or("");
                        let client_secret = cfg.client_secret.as_deref().unwrap_or("");
                        let secret_display = if client_secret.len() > 8 {
                            format!("{}...", &client_secret[..8])
                        } else {
                            "***".to_string()
                        };
                        serde_json::json!({
                            "auth_method": "client_credentials",
                            "api_url": cfg.api_url,
                            "client_id": client_id,
                            "client_secret": secret_display,
                            "token": cfg.token.as_ref().map(|t| serde_json::json!({
                                "expires_at": t.expires_at,
                                "has_token": true,
                            })),
                        })
                    }
                };
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
