use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;

#[derive(Subcommand)]
pub enum CustomDomainCommands {
    /// List all custom domains.
    ///
    /// Returns: { custom_domains: [...], total, limit, offset }
    List {
        /// Maximum number of results (1-100)
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Get custom domain details including DNS verification records.
    ///
    /// Returns: { id, tenant_id, custom_domain, status, ssl_status, verification_records: [...], created_at, updated_at }
    /// Status values: "active", "pending", "error", "loading"
    Get {
        /// Custom domain public ID
        #[arg(long)]
        id: String,
    },
    /// Create a custom domain and attach it to a tenant.
    ///
    /// Required JSON fields:
    ///   - tenant_id (string): Tenant public ID to attach the domain to
    ///   - domain (string): The custom domain hostname (e.g. "auth.example.com")
    ///
    /// Returns the domain with DNS verification records that must be configured at your DNS provider.
    ///
    /// Example: --json '{"tenant_id": "tenant_abc123", "domain": "auth.example.com"}'
    Create {
        /// JSON request body (see `authalla custom-domain schema create` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Trigger re-verification of DNS records for a custom domain.
    /// Use this after configuring DNS records at your domain registrar.
    Verify {
        /// Custom domain public ID
        #[arg(long)]
        id: String,
    },
    /// Print the JSON schema for the create operation.
    ///
    /// Usage: authalla custom-domain schema create
    Schema {
        /// Operation: "create"
        operation: String,
    },
}

pub fn run(cmd: CustomDomainCommands) -> Result<()> {
    match cmd {
        CustomDomainCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        CustomDomainCommands::List { limit, offset } => {
            let result = api.get_with_query(
                "/api/v1/custom-domains",
                &[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ],
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomDomainCommands::Get { id } => {
            let result = api.get(&format!("/api/v1/custom-domains/{}", id))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomDomainCommands::Create { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.post("/api/v1/custom-domains", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomDomainCommands::Verify { id } => {
            let result = api.post(
                &format!("/api/v1/custom-domains/{}/verify", id),
                &serde_json::json!({}),
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomDomainCommands::Schema { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let schema = match operation {
        "create" => serde_json::json!({
            "description": "Create a custom domain and attach it to a tenant. Returns DNS verification records.",
            "required": ["tenant_id", "domain"],
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant public ID to attach the domain to",
                    "maxLength": 100,
                    "example": "tenant_abc123xyz"
                },
                "domain": {
                    "type": "string",
                    "format": "hostname",
                    "description": "The custom domain hostname",
                    "maxLength": 253,
                    "example": "auth.example.com"
                }
            },
            "example": {
                "tenant_id": "tenant_abc123xyz",
                "domain": "auth.example.com"
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'create'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
