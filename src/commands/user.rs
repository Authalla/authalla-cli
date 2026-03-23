use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;

#[derive(Subcommand)]
pub enum UserCommands {
    /// List users in the tenant associated with the access token.
    ///
    /// Returns: { users: [...], total, limit, offset }
    List {
        /// Maximum number of results (1-1000)
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Number of results to skip
        #[arg(long, default_value = "0")]
        offset: u32,
        /// Search by email or name (case-insensitive substring match)
        #[arg(long)]
        search: Option<String>,
    },
    /// Get a user by ID.
    ///
    /// Returns: { id, email, name, email_verified, status, last_login, scopes, created_at, updated_at }
    Get {
        /// User public ID (e.g. user_abc123xyz)
        #[arg(long)]
        id: String,
    },
    /// Create a new user in the tenant.
    ///
    /// Required JSON fields:
    ///   - email (string): User email address
    ///   - name (string): User display name
    ///
    /// Optional JSON fields:
    ///   - scopes (string[]): Custom scopes granted to this user (max 100, each max 255 chars)
    ///
    /// Example: --json '{"email": "jane@example.com", "name": "Jane Doe"}'
    Create {
        /// JSON request body (see `authalla user schema create` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Update an existing user.
    ///
    /// All JSON fields are optional:
    ///   - email (string): New email address
    ///   - name (string): New display name
    ///   - status (string): "active", "inactive", or "suspended"
    ///   - scopes (string[]): Custom scopes granted to this user
    ///
    /// Example: --json '{"status": "suspended"}'
    Update {
        /// User public ID (e.g. user_abc123xyz)
        #[arg(long)]
        id: String,
        /// JSON request body (see `authalla user schema update` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Delete a user by ID.
    Delete {
        /// User public ID (e.g. user_abc123xyz)
        #[arg(long)]
        id: String,
    },
    /// Print the JSON schema for create or update operations.
    ///
    /// Usage: authalla user schema create
    ///        authalla user schema update
    Schema {
        /// Operation: "create" or "update"
        operation: String,
    },
}

pub fn run(cmd: UserCommands) -> Result<()> {
    match cmd {
        UserCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        UserCommands::List {
            limit,
            offset,
            search,
        } => {
            let mut query = vec![
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
            ];
            if let Some(ref s) = search {
                query.push(("search", s.clone()));
            }
            let query_refs: Vec<(&str, &str)> =
                query.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let result = api.get_with_query("/api/v1/users", &query_refs)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        UserCommands::Get { id } => {
            let result = api.get(&format!("/api/v1/users/{}", id))?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        UserCommands::Create { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.post("/api/v1/users", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        UserCommands::Update { id, json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.put(&format!("/api/v1/users/{}", id), &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        UserCommands::Delete { id } => {
            api.delete(&format!("/api/v1/users/{}", id))?;
            println!("{{\"deleted\": true}}");
        }
        UserCommands::Schema { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let schema = match operation {
        "create" => serde_json::json!({
            "description": "Create a new user in the tenant",
            "required": ["email", "name"],
            "properties": {
                "email": {
                    "type": "string",
                    "format": "email",
                    "maxLength": 254,
                    "description": "User email address",
                    "example": "jane@example.com"
                },
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 255,
                    "description": "User display name",
                    "example": "Jane Doe"
                },
                "scopes": {
                    "type": "array",
                    "description": "Custom scopes granted to this user (used to gate non-standard OAuth2 scopes)",
                    "maxItems": 100,
                    "items": { "type": "string", "maxLength": 255 },
                    "example": ["premium", "beta-access"]
                }
            },
            "example": {
                "email": "jane@example.com",
                "name": "Jane Doe",
                "scopes": ["premium"]
            }
        }),
        "update" => serde_json::json!({
            "description": "Update an existing user (all fields optional)",
            "required": [],
            "properties": {
                "email": {
                    "type": "string",
                    "format": "email",
                    "maxLength": 254,
                    "description": "New email address"
                },
                "name": {
                    "type": "string",
                    "maxLength": 255,
                    "description": "New display name"
                },
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "suspended"],
                    "description": "User status"
                },
                "scopes": {
                    "type": "array",
                    "description": "Custom scopes granted to this user",
                    "maxItems": 100,
                    "items": { "type": "string", "maxLength": 255 }
                }
            },
            "example": {
                "name": "Jane Smith",
                "status": "active"
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'create' or 'update'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
