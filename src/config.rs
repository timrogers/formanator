//! Persistent configuration stored at `~/.formanatorrc.json`.
//!
//! This matches the file format used by the original Node.js implementation, so
//! the two clients can share the same login state.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::keychain;

const CONFIG_FILENAME: &str = ".formanatorrc.json";

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
    let parsed: Config = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse config file at {}", path.display()))?;
    Ok(Some(parsed))
}

/// Return the saved access token, if any.
///
/// On macOS, this checks the Keychain first, then falls back to the config file.
/// On other platforms, this only checks the config file.
pub fn get_access_token() -> Result<Option<String>> {
    // Try to get token from Keychain first (on macOS)
    if let Some(token) = keychain::get_access_token()? {
        // Filter out empty tokens for consistency
        if !token.is_empty() {
            return Ok(Some(token));
        }
    }

    // Fall back to config file
    Ok(read_config()?
        .map(|c| c.access_token)
        .filter(|t| !t.is_empty()))
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

/// Persist the given config to disk and store the token in the system keychain if on macOS.
///
/// On macOS: token goes to Keychain, other config (email, timestamps) goes to JSON file
/// On other platforms: all config including token goes to JSON file (for backward compatibility)
pub fn store_config(config: &Config) -> Result<()> {
    // Store token in Keychain on macOS
    #[cfg(target_os = "macos")]
    if !config.access_token.is_empty() {
        keychain::store_access_token(&config.access_token)?;
    }

    let path = config_path()?;

    // Determine what to write to file based on platform
    #[cfg(target_os = "macos")]
    let config_to_write = Config {
        access_token: String::new(),
        email: config.email.clone(),
        last_update_check_timestamp: config.last_update_check_timestamp,
    };

    #[cfg(not(target_os = "macos"))]
    let config_to_write = config.clone();

    let serialised = serde_json::to_string(&config_to_write)?;
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
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"accessToken\":\"tok\""), "{json}");
        // `email` is `None`, so it should be skipped.
        assert!(!json.contains("email"), "{json}");
    }

    #[test]
    fn config_serializes_email_when_present() {
        let config = Config {
            access_token: "tok".to_string(),
            email: Some("user@example.com".to_string()),
            ..Config::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"email\":\"user@example.com\""), "{json}");
    }

    #[test]
    fn config_round_trips_through_json() {
        let original = Config {
            access_token: "tok".to_string(),
            email: Some("user@example.com".to_string()),
            last_update_check_timestamp: Some(1_700_000_000),
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(
            json.contains("\"lastUpdateCheckTimestamp\":1700000000"),
            "{json}"
        );
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, original.access_token);
        assert_eq!(parsed.email, original.email);
        assert_eq!(
            parsed.last_update_check_timestamp,
            original.last_update_check_timestamp
        );
    }

    #[test]
    #[serial_test::serial]
    fn store_config_separates_token_from_file() {
        // Set a custom config path for testing
        let tmpdir = tempfile::tempdir().unwrap();
        let config_path = tmpdir.path().join(".formanatorrc.json");
        unsafe {
            std::env::set_var("FORMANATOR_CONFIG_PATH", &config_path);
        }

        let config = Config {
            access_token: "secret-token-12345".to_string(),
            email: Some("user@example.com".to_string()),
            last_update_check_timestamp: Some(1_700_000_000),
        };

        // Store config
        store_config(&config).unwrap();

        // Read back the file
        let file_content = std::fs::read_to_string(&config_path).unwrap();

        // Platform-specific behavior:
        // On macOS: token should NOT be in file, only metadata
        // On non-macOS: token SHOULD be in file for backward compatibility
        #[cfg(target_os = "macos")]
        {
            assert!(!file_content.contains("secret-token-12345"));
            assert!(file_content.contains("user@example.com"));
            assert!(file_content.contains("1700000000"));
        }

        #[cfg(not(target_os = "macos"))]
        {
            assert!(file_content.contains("secret-token-12345"));
            assert!(file_content.contains("user@example.com"));
            assert!(file_content.contains("1700000000"));
        }

        // Clean up
        unsafe {
            std::env::remove_var("FORMANATOR_CONFIG_PATH");
        }
    }

    #[test]
    fn resolve_access_token_prefers_explicit_value() {
        // When an explicit value is provided, the saved config is not consulted.
        let token = resolve_access_token(Some("from-cli")).unwrap();
        assert_eq!(token, "from-cli");
    }
}
