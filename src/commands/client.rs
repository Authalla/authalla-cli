use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;

#[derive(Subcommand)]
pub enum ClientCommands {
    /// List OAuth2 clients.
    ///
    /// Returns: { clients: [...], total, limit, offset }
    List {
        /// Maximum number of results (1-100)
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value = "0")]
        offset: u32,
        /// Filter by tenant public ID (omit for all tenants)
        #[arg(long)]
        tenant_id: Option<String>,
    },
    /// Get OAuth2 client details including redirect URIs and scopes.
    ///
    /// Returns: { id, name, application_type, tenant_id, tenant_name, is_public, status,
    ///            allowed_scopes, redirect_uris, allowed_logout_uris, created_at, updated_at }
    Get {
        /// Client public ID
        #[arg(long)]
        id: String,
    },
    /// Create a new OAuth2 client.
    ///
    /// The client secret is only returned once on creation for confidential clients (web, backend types).
    ///
    /// Required JSON fields:
    ///   - name (string): Client display name
    ///   - tenant_id (string): Tenant public ID
    ///   - application_type (string): "spa", "native", "web", or "backend"
    ///       spa    = Public client, authorization_code + refresh_token
    ///       native = Public client, authorization_code + refresh_token
    ///       web    = Confidential client, authorization_code + refresh_token
    ///       backend = Confidential client, client_credentials (machine-to-machine)
    ///
    /// Optional JSON fields:
    ///   - redirect_uris (string[]): OAuth2 redirect URIs (max 50, each max 2048 chars)
    ///   - allowed_logout_uris (string[]): Post-logout redirect URIs (max 50)
    ///   - scopes (string[]): Allowed OAuth2 scopes (defaults to openid, profile, email)
    ///   - homepage_url (string): Application homepage URL
    ///
    /// Example: --json '{"name": "My Web App", "tenant_id": "tenant_abc123", "application_type": "web", "redirect_uris": ["https://app.example.com/callback"]}'
    Create {
        /// JSON request body (see `authalla client schema create` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Print the JSON schema for the create operation.
    ///
    /// Usage: authalla client schema create
    Schema {
        /// Operation: "create"
        operation: String,
    },
}

pub fn run(cmd: ClientCommands) -> Result<()> {
    match cmd {
        ClientCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        ClientCommands::List {
            limit,
            offset,
            tenant_id,
        } => {
            let mut query = vec![
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
            ];
            if let Some(ref tid) = tenant_id {
                query.push(("tenant_id", tid.clone()));
            }
            let query_refs: Vec<(&str, &str)> =
                query.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let result = api.get_with_query("/api/v1/clients", &query_refs)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        ClientCommands::Get { id } => {
            let result = api.get(&format!("/api/v1/clients/{}", id))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        ClientCommands::Create { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.post("/api/v1/clients", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        ClientCommands::Schema { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let schema = match operation {
        "create" => serde_json::json!({
            "description": "Create a new OAuth2 client. The client_secret is only returned once on creation for confidential clients (web, backend).",
            "required": ["name", "tenant_id", "application_type"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Client display name",
                    "minLength": 1,
                    "maxLength": 255,
                    "example": "My Web App"
                },
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant public ID",
                    "maxLength": 100,
                    "example": "tenant_abc123xyz"
                },
                "application_type": {
                    "type": "string",
                    "enum": ["spa", "native", "web", "backend"],
                    "description": "Application type. spa/native = public client (auth code + refresh). web = confidential (auth code + refresh). backend = confidential (client credentials, machine-to-machine)."
                },
                "redirect_uris": {
                    "type": "array",
                    "description": "OAuth2 redirect URIs (required for spa, native, web types)",
                    "maxItems": 50,
                    "items": { "type": "string", "maxLength": 2048 },
                    "example": ["https://app.example.com/callback"]
                },
                "allowed_logout_uris": {
                    "type": "array",
                    "description": "Allowed post-logout redirect URIs",
                    "maxItems": 50,
                    "items": { "type": "string", "maxLength": 2048 },
                    "example": ["https://app.example.com"]
                },
                "scopes": {
                    "type": "array",
                    "description": "Allowed OAuth2 scopes (defaults to openid, profile, email)",
                    "maxItems": 100,
                    "items": { "type": "string", "maxLength": 255 },
                    "example": ["openid", "profile", "email"]
                },
                "homepage_url": {
                    "type": "string",
                    "description": "Application homepage URL",
                    "maxLength": 2048,
                    "example": "https://app.example.com"
                }
            },
            "example": {
                "name": "My Web App",
                "tenant_id": "tenant_abc123xyz",
                "application_type": "web",
                "redirect_uris": ["https://app.example.com/callback"],
                "allowed_logout_uris": ["https://app.example.com"],
                "scopes": ["openid", "profile", "email"]
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'create'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
