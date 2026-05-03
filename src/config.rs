//! Persistent configuration stored at `~/.formanator.toml` (TOML format).
//!
//! This matches the file location used by previous clients but uses TOML for
//! readability and portability. Existing on-disk JSON files are not migrated by
//! this change.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &str = ".formanator.toml";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub email: Option<String>,
    /// Unix timestamp (seconds) of the last auto-update check. Persisted so we
    /// only check at most once per day across CLI invocations.
    #[serde(
        rename = "lastUpdateCheckTimestamp",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub last_update_check_timestamp: Option<u64>,
}

fn config_path() -> Result<PathBuf> {
    // Allow the path to be overridden for testing or custom deployments.
    if let Some(path) = std::env::var_os("FORMANATOR_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    let home = dirs::home_dir().context("Could not determine your home directory")?;
    Ok(home.join(CONFIG_FILENAME))
}

/// Read the saved config from disk, returning `None` if the file does not exist.
pub fn read_config() -> Result<Option<Config>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file at {}", path.display()))?;
    let parsed: Config = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse config file at {}", path.display()))?;
    Ok(Some(parsed))
}

/// Return the saved access token, if any.
pub fn get_access_token() -> Result<Option<String>> {
    Ok(read_config()?.map(|c| c.access_token))
}

/// Resolve an access token from an explicit CLI/env value, falling back to the
/// saved config file.
pub fn resolve_access_token(explicit: Option<&str>) -> Result<String> {
    if let Some(token) = explicit {
        return Ok(token.to_string());
    }
    match get_access_token()? {
        Some(t) if !t.is_empty() => Ok(t),
        _ => anyhow::bail!("You aren't logged in to Forma. Please run `formanator login` first."),
    }
}

/// Persist the given config to disk.
pub fn store_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    let serialised = toml::to_string(config)?;
    fs::write(&path, serialised)
        .with_context(|| format!("Failed to write config file at {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serializes_with_camelcase_access_token_and_omits_email() {
        let config = Config {
            access_token: "tok".to_string(),
            email: None,
            ..Config::default()
        };
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("accessToken = \"tok\""), "{toml}");
        // `email` is `None`, so it should be skipped.
        assert!(!toml.contains("email"), "{toml}");
    }

    #[test]
    fn config_serializes_email_when_present() {
        let config = Config {
            access_token: "tok".to_string(),
            email: Some("user@example.com".to_string()),
            ..Config::default()
        };
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("email = \"user@example.com\""), "{toml}");
    }

    #[test]
    fn config_round_trips_through_json() {
        let original = Config {
            access_token: "tok".to_string(),
            email: Some("user@example.com".to_string()),
            last_update_check_timestamp: Some(1_700_000_000),
        };
        let toml = toml::to_string(&original).unwrap();
        assert!(
            toml.contains("lastUpdateCheckTimestamp = 1700000000"),
            "{toml}"
        );
        let parsed: Config = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.access_token, original.access_token);
        assert_eq!(parsed.email, original.email);
        assert_eq!(
            parsed.last_update_check_timestamp,
            original.last_update_check_timestamp
        );
    }

    #[test]
    fn resolve_access_token_prefers_explicit_value() {
        // When an explicit value is provided, the saved config is not consulted.
        let token = resolve_access_token(Some("from-cli")).unwrap();
        assert_eq!(token, "from-cli");
    }
}
