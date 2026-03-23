use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;

#[derive(Subcommand)]
pub enum SocialLoginCommands {
    /// List all configured social login providers.
    ///
    /// Returns: { social_logins: [...], total, limit, offset }
    /// Each entry: { id, name, provider_type, client_id, status, created_at, updated_at }
    List {
        /// Maximum number of results (1-100)
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Create a social login provider (Google, GitHub, Apple, or Microsoft).
    ///
    /// Required JSON fields:
    ///   - name (string): Display name for the provider
    ///   - provider_type (string): "google", "github", "apple", or "microsoft"
    ///   - client_id (string): OAuth2 client ID from the provider
    ///   - client_secret (string): OAuth2 client secret from the provider
    ///
    /// Optional JSON fields:
    ///   - tenant_ids (string[]): Tenant public IDs to attach this provider to (max 100)
    ///
    /// Example: --json '{"name": "Google Login", "provider_type": "google", "client_id": "xxx.apps.googleusercontent.com", "client_secret": "GOCSPX-xxx", "tenant_ids": ["tenant_abc123"]}'
    Create {
        /// JSON request body (see `authalla social-login schema create` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Print the JSON schema for the create operation.
    ///
    /// Usage: authalla social-login schema create
    Schema {
        /// Operation: "create"
        operation: String,
    },
}

pub fn run(cmd: SocialLoginCommands) -> Result<()> {
    match cmd {
        SocialLoginCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        SocialLoginCommands::List { limit, offset } => {
            let result = api.get_with_query(
                "/api/v1/social-logins",
                &[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ],
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        SocialLoginCommands::Create { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.post("/api/v1/social-logins", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        SocialLoginCommands::Schema { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let schema = match operation {
        "create" => serde_json::json!({
            "description": "Configure a new social login provider (Google, GitHub, Apple, Microsoft).",
            "required": ["name", "provider_type", "client_id", "client_secret"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Display name for the provider",
                    "minLength": 1,
                    "maxLength": 255,
                    "example": "Google Login"
                },
                "provider_type": {
                    "type": "string",
                    "enum": ["google", "github", "apple", "microsoft"],
                    "description": "Social login provider type"
                },
                "client_id": {
                    "type": "string",
                    "description": "OAuth2 client ID from the social provider",
                    "minLength": 1,
                    "maxLength": 512,
                    "example": "123456789.apps.googleusercontent.com"
                },
                "client_secret": {
                    "type": "string",
                    "description": "OAuth2 client secret from the social provider",
                    "minLength": 1,
                    "maxLength": 2048
                },
                "tenant_ids": {
                    "type": "array",
                    "description": "Tenant public IDs to attach this provider to",
                    "maxItems": 100,
                    "items": { "type": "string", "maxLength": 100 },
                    "example": ["tenant_abc123xyz"]
                }
            },
            "example": {
                "name": "Google Login",
                "provider_type": "google",
                "client_id": "123456789.apps.googleusercontent.com",
                "client_secret": "GOCSPX-xxx",
                "tenant_ids": ["tenant_abc123xyz"]
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'create'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
