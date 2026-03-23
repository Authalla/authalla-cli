use anyhow::{Context, Result};
use clap::Subcommand;

use crate::api::ApiClient;

#[derive(Subcommand)]
pub enum ThemeCommands {
    /// Get current theme settings.
    ///
    /// Returns: { logo_url, primary_color, secondary_color, secondary_button_text_color,
    ///            background_color, box_background_color, text_color, dark: { ... } }
    Get,
    /// Update theme settings. All fields are optional — only provided fields are changed.
    ///
    /// Available JSON fields (all optional, colors in hex e.g. "#9333ea"):
    ///   - primary_color (string): Primary brand color
    ///   - secondary_color (string): Secondary brand color
    ///   - secondary_button_text_color (string): Text color for secondary buttons
    ///   - background_color (string): Page background color
    ///   - box_background_color (string): Card/box background color
    ///   - text_color (string): Main text color
    ///   - dark (object): Dark mode overrides with the same color fields above
    ///
    /// Example: --json '{"primary_color": "#9333ea", "dark": {"primary_color": "#a855f7"}}'
    Update {
        /// JSON request body (see `authalla theme schema update` for full schema)
        #[arg(long)]
        json: String,
    },
    /// Print the JSON schema for the update operation.
    ///
    /// Usage: authalla theme schema update
    Schema {
        /// Operation: "update"
        operation: String,
    },
}

pub fn run(cmd: ThemeCommands) -> Result<()> {
    match cmd {
        ThemeCommands::Schema { operation } => {
            print_schema(&operation)?;
            return Ok(());
        }
        _ => {}
    }

    let api = ApiClient::new()?;

    match cmd {
        ThemeCommands::Get => {
            let result = api.get("/api/v1/theme")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        ThemeCommands::Update { json } => {
            let body: serde_json::Value =
                serde_json::from_str(&json).context("Invalid JSON input")?;
            let result = api.put("/api/v1/theme", &body)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        ThemeCommands::Schema { .. } => unreachable!(),
    }
    Ok(())
}

fn print_schema(operation: &str) -> Result<()> {
    let color_field = serde_json::json!({
        "type": "string",
        "pattern": "^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$",
        "maxLength": 7,
        "example": "#9333ea"
    });

    let color_properties = serde_json::json!({
        "primary_color": {
            "type": "string",
            "description": "Primary brand color (hex)",
            "pattern": "^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$",
            "example": "#9333ea"
        },
        "secondary_color": {
            "type": "string",
            "description": "Secondary brand color (hex)",
            "pattern": color_field["pattern"]
        },
        "secondary_button_text_color": {
            "type": "string",
            "description": "Text color for secondary buttons (hex)",
            "pattern": color_field["pattern"]
        },
        "background_color": {
            "type": "string",
            "description": "Page background color (hex)",
            "pattern": color_field["pattern"]
        },
        "box_background_color": {
            "type": "string",
            "description": "Card/box background color (hex)",
            "pattern": color_field["pattern"]
        },
        "text_color": {
            "type": "string",
            "description": "Main text color (hex)",
            "pattern": color_field["pattern"]
        }
    });

    let schema = match operation {
        "update" => serde_json::json!({
            "description": "Update theme settings. All fields are optional — only provided fields are changed. Colors must be hex format.",
            "required": [],
            "properties": {
                "primary_color": color_properties["primary_color"],
                "secondary_color": color_properties["secondary_color"],
                "secondary_button_text_color": color_properties["secondary_button_text_color"],
                "background_color": color_properties["background_color"],
                "box_background_color": color_properties["box_background_color"],
                "text_color": color_properties["text_color"],
                "dark": {
                    "type": "object",
                    "description": "Dark mode color overrides (same fields as above)",
                    "properties": color_properties
                }
            },
            "example": {
                "primary_color": "#9333ea",
                "background_color": "#ffffff",
                "dark": {
                    "primary_color": "#a855f7",
                    "background_color": "#1a1a2e"
                }
            }
        }),
        _ => anyhow::bail!("Unknown operation '{}'. Use 'update'.", operation),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
