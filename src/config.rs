use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub token: Option<CachedToken>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: i64,
}

fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("authalla");
    Ok(config_dir.join("config.json"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Could not read config at {}. Run `authalla config set` first.", path.display()))?;
    let config: Config = serde_json::from_str(&contents)
        .context("Invalid config file format")?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(config)?;
    fs::write(&path, &contents)?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
