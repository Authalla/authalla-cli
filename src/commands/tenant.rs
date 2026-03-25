use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;
use crate::config;

#[derive(Subcommand)]
pub enum TenantCommands {
    /// Switch the active tenant for subsequent CLI commands.
    Select {
        /// Tenant public ID to switch to
        id: String,
    },
    /// List all tenants.
    ///
    /// Returns: { tenants: [...], total, limit, offset }
    List {
        /// Maximum number of results (1-1000)
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Get a tenant by ID.
    ///
    /// Returns: { id, name, status, allow_registration, created_at, updated_at }
    Get {
        /// Tenant public ID (e.g. tenant_abc123xyz)
        #[arg(long)]
        id: String,
    },
    /// Create a new tenant.
    ///
    /// Required JSON fields:
    ///   - name (string): Human-readable tenant name
    ///   - allow_registration (bool): Whether new users can self-register
    ///
    /// Example: --json '{"name": "Production", "allow_registration": true}'
    Create {
        /// JSON request body (see `authalla tenant schema create` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Update an existing tenant.
    ///
    /// Required JSON fields:
    ///   - name (string): Updated tenant name
    ///   - allow_registration (bool): Whether to allow registration
    ///
    /// Optional JSON fields:
    ///   - auth_methods (string[]): Enabled auth methods. Values: "magic_link", "passkeys", "social_logins", "enterprise_sso"
    ///
    /// Example: --json '{"name": "Production", "allow_registration": false, "auth_methods": ["magic_link", "passkeys"]}'
    Update {
        /// Tenant public ID (e.g. tenant_abc123xyz)
        #[arg(long)]
        id: String,
        /// JSON request body (see `authalla tenant schema update` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Delete a tenant by ID.
    Delete {
        /// Tenant public ID (e.g. tenant_abc123xyz)
        #[arg(long)]
        id: String,
    },
    /// Print the JSON schema for create or update operations.
    ///
    /// Usage: authalla tenant schema create
    ///        authalla tenant schema update
    Schema {
        /// Operation: "create" or "update"
        operation: String,
    },
}

pub fn run(cmd: TenantCommands) -> Result<()> {
    match cmd {
        TenantCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        TenantCommands::Select { id } => {
            let mut cfg = config::load()?;
            cfg.tenant_id = Some(id.clone());
            config::save(&cfg)?;
            eprintln!("Active tenant: {}", id);
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        TenantCommands::List { limit, offset } => {
            let result = api.get_with_query(
                "/api/v1/tenants",
                &[
                    ("limit", &limit.to_string()),
                    ("offset", &offset.to_string()),
                ],
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        TenantCommands::Get { id } => {
            let result = api.get(&format!("/api/v1/tenants/{}", id))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        TenantCommands::Create { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.post("/api/v1/tenants", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        TenantCommands::Update { id, json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.put(&format!("/api/v1/tenants/{}", id), &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        TenantCommands::Delete { id } => {
            api.delete(&format!("/api/v1/tenants/{}", id))?;
            println!("{{\"deleted\": true}}");
        }
        TenantCommands::Schema { .. } | TenantCommands::Select { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let schema = match operation {
        "create" => serde_json::json!({
            "description": "Create a new tenant",
            "required": ["name", "allow_registration"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Human-readable name for the tenant",
                    "minLength": 1,
                    "example": "Production Environment"
                },
                "allow_registration": {
                    "type": "boolean",
                    "description": "Whether new users can self-register in this tenant",
                    "example": true
                }
            },
            "example": {
                "name": "Production Environment",
                "allow_registration": true
            }
        }),
        "update" => serde_json::json!({
            "description": "Update an existing tenant",
            "required": ["name", "allow_registration"],
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Updated name for the tenant",
                    "minLength": 1
                },
                "allow_registration": {
                    "type": "boolean",
                    "description": "Whether to allow new user registration"
                },
                "auth_methods": {
                    "type": "array",
                    "description": "Authentication methods to enable for this tenant",
                    "items": {
                        "type": "string",
                        "enum": ["magic_link", "passkeys", "social_logins", "enterprise_sso"]
                    }
                }
            },
            "example": {
                "name": "Production Environment",
                "allow_registration": false,
                "auth_methods": ["magic_link", "passkeys"]
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'create' or 'update'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
