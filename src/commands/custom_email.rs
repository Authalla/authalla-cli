use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;

#[derive(Subcommand)]
pub enum CustomEmailCommands {
    /// List all custom email domains.
    ///
    /// Returns: { custom_emails: [...], total, limit, offset }
    List {
        /// Maximum number of results (1-100)
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Get custom email domain details including DNS records for verification.
    ///
    /// Returns: { id, tenant_id, email_domain, sender_email, sender_verified, is_active,
    ///            verification_status, dns_records: [...], created_at, updated_at }
    /// verification_status values: "pending", "verified", "error"
    Get {
        /// Custom email link ID
        #[arg(long)]
        id: String,
    },
    /// Create a custom email domain for sending authentication emails.
    ///
    /// Required JSON fields:
    ///   - tenant_id (string): Tenant public ID
    ///   - email_domain (string): Email domain (e.g. "mail.example.com")
    ///
    /// Optional JSON fields:
    ///   - sender_email (string): Sender email address (defaults to noreply@{email_domain})
    ///   - sender_name (string): Sender display name (defaults to "Authalla")
    ///
    /// Returns the domain with DNS records that must be configured at your DNS provider.
    ///
    /// Example: --json '{"tenant_id": "tenant_abc123", "email_domain": "mail.example.com"}'
    Create {
        /// JSON request body (see `authalla custom-email schema create` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Trigger re-verification of DNS records for a custom email domain.
    /// Use this after configuring DNS records at your domain registrar.
    Verify {
        /// Custom email link ID
        #[arg(long)]
        id: String,
    },
    /// Print the JSON schema for the create operation.
    ///
    /// Usage: authalla custom-email schema create
    Schema {
        /// Operation: "create"
        operation: String,
    },
}

pub fn run(cmd: CustomEmailCommands) -> Result<()> {
    match cmd {
        CustomEmailCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        CustomEmailCommands::List { limit, offset } => {
            let result = api.get_with_query(
                "/api/v1/custom-emails",
                &[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ],
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomEmailCommands::Get { id } => {
            let result = api.get(&format!("/api/v1/custom-emails/{}", id))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomEmailCommands::Create { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.post("/api/v1/custom-emails", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomEmailCommands::Verify { id } => {
            let result = api.post(
                &format!("/api/v1/custom-emails/{}/verify", id),
                &serde_json::json!({}),
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        CustomEmailCommands::Schema { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let schema = match operation {
        "create" => serde_json::json!({
            "description": "Create a custom email domain for sending authentication emails. Returns DNS records for verification.",
            "required": ["tenant_id", "email_domain"],
            "properties": {
                "tenant_id": {
                    "type": "string",
                    "description": "Tenant public ID",
                    "maxLength": 100,
                    "example": "tenant_abc123xyz"
                },
                "email_domain": {
                    "type": "string",
                    "format": "hostname",
                    "description": "Email domain (e.g. mail.example.com)",
                    "maxLength": 253,
                    "example": "mail.example.com"
                },
                "sender_email": {
                    "type": "string",
                    "format": "email",
                    "description": "Sender email address (defaults to noreply@{email_domain})",
                    "maxLength": 254,
                    "example": "noreply@mail.example.com"
                },
                "sender_name": {
                    "type": "string",
                    "description": "Sender display name (defaults to \"Authalla\")",
                    "maxLength": 255,
                    "example": "My App"
                }
            },
            "example": {
                "tenant_id": "tenant_abc123xyz",
                "email_domain": "mail.example.com",
                "sender_email": "noreply@mail.example.com",
                "sender_name": "My App"
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'create'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
