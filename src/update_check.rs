//! Daily check for newer releases of `formanator` on GitHub.
//!
//! This is modelled after the auto-update check in
//! [`timrogers/litra-rs`](https://github.com/timrogers/litra-rs):
//!
//! * The check runs at most once per day. The timestamp of the last check is
//!   persisted in the existing `~/.formanator.toml` config file as
//!   `lastUpdateCheckTimestamp`.
//! * Only releases that are at least 72 hours old are considered, to give
//!   freshly published releases a chance to be yanked or fixed before users are
//!   nudged to upgrade.
//! * Network failures and timeouts are silently ignored so the CLI keeps
//!   working when offline.
//! * Set the `FORMANATOR_DISABLE_UPDATE_CHECK` environment variable to any
//!   value to skip the check entirely.

use std::time::Duration;

use colored::Colorize;
use serde::Deserialize;

use crate::config::{Config, read_config, store_config};

/// The current version of the CLI, extracted from `Cargo.toml`.
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub API URL for fetching releases (list endpoint).
const GITHUB_API_URL: &str = "https://api.github.com/repos/timrogers/formanator/releases";

/// Timeout for update check requests in seconds.
const UPDATE_CHECK_TIMEOUT_SECS: u64 = 2;

/// Number of seconds in a day (24 hours).
const SECONDS_PER_DAY: u64 = 86_400;

/// Environment variable used to disable the update check entirely.
pub const DISABLE_UPDATE_CHECK_ENV: &str = "FORMANATOR_DISABLE_UPDATE_CHECK";

/// Response structure for the GitHub releases API.
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    published_at: String,
}

/// Returns the current Unix timestamp in seconds.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Returns `true` when at least one day has passed since the last update check
/// recorded in `config`.
fn should_check_for_updates(config: &Config) -> bool {
    let Some(last_check) = config.last_update_check_timestamp else {
        return true;
    };
    current_timestamp().saturating_sub(last_check) >= SECONDS_PER_DAY
}

/// Returns `true` when the release timestamp `published_at` is at least 72
/// hours in the past.
fn is_release_old_enough(published_at: &str) -> bool {
    use chrono::{DateTime, Duration as ChronoDuration, Utc};

    let Ok(release_time) = DateTime::parse_from_rfc3339(published_at) else {
        return false;
    };
    let cutoff = Utc::now() - ChronoDuration::hours(72);
    release_time < cutoff
}

/// Compares two semantic version strings and returns `true` when `latest` is
/// strictly newer than `current`. Versions with fewer than three components
/// are accepted; missing components are treated as `0`.
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        match parts.len() {
            n if n >= 3 => Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            )),
            2 => Some((parts[0].parse().ok()?, parts[1].parse().ok()?, 0)),
            1 => Some((parts[0].parse().ok()?, 0, 0)),
            _ => None,
        }
    };

    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Generates the user-facing update notification message in yellow.
fn format_update_message(latest_version: &str) -> String {
    format!(
        "A new version of formanator is available: {} (current: v{})\n\
         If you installed Formanator from Homebrew, you can upgrade by running `brew upgrade formanator`.\n\
         If you installed it via Cargo, you can upgrade by running `cargo install formanator`.\n\
         Otherwise, you can download the latest release at https://github.com/timrogers/formanator/releases/tag/{}",
        latest_version, CURRENT_VERSION, latest_version
    )
    .yellow()
    .to_string()
}

/// Fetches releases from GitHub and returns the highest version tag that is
/// newer than the current version and at least 72 hours old, if any.
///
/// The check is throttled to at most once per day and can be disabled via the
/// [`DISABLE_UPDATE_CHECK_ENV`] environment variable. Network and I/O failures
/// are swallowed so they cannot disrupt normal CLI operation.
fn check_for_updates() -> Option<String> {
    if std::env::var(DISABLE_UPDATE_CHECK_ENV).is_ok() {
        return None;
    }

    // Read the persisted config; treat any read/parse error as "no config yet".
    // We only persist the timestamp when a config file already exists, so an
    // update check never creates the config file on its own.
    let existing = read_config().ok().flatten();
    let mut config = existing.clone().unwrap_or_default();
    if !should_check_for_updates(&config) {
        return None;
    }

    // Update the timestamp first, so a hung or failing API doesn't cause us to
    // retry on every invocation. Errors writing the config are intentionally
    // ignored, and we skip writing entirely when no config file exists yet.
    config.last_update_check_timestamp = Some(current_timestamp());
    if existing.is_some() {
        let _ = store_config(&config);
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(UPDATE_CHECK_TIMEOUT_SECS))
        .user_agent(format!("formanator/{CURRENT_VERSION}"))
        .build()
    {
        Ok(client) => client,
        Err(_) => return None,
    };

    let response = match client
        .get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
    {
        Ok(response) => response,
        Err(e) => {
            if e.is_timeout() {
                eprintln!(
                    "Warning: Update check timed out after {UPDATE_CHECK_TIMEOUT_SECS} seconds"
                );
            }
            return None;
        }
    };

    let releases: Vec<GitHubRelease> = match response.json() {
        Ok(releases) => releases,
        Err(_) => return None,
    };

    let mut best_version: Option<String> = None;
    for release in releases {
        if !is_release_old_enough(&release.published_at) {
            continue;
        }
        let release_version = release.tag_name.trim_start_matches('v');
        if !is_newer_version(release_version, CURRENT_VERSION) {
            continue;
        }
        match &best_version {
            None => best_version = Some(release.tag_name),
            Some(current_best) => {
                if is_newer_version(release_version, current_best.trim_start_matches('v')) {
                    best_version = Some(release.tag_name);
                }
            }
        }
    }

    best_version
}

/// Performs the update check and prints a yellow notification to stderr when a
/// newer release is available. Errors are intentionally swallowed.
pub fn print_update_notification() {
    if let Some(latest_version) = check_for_updates() {
        eprintln!("{}\n", format_update_message(&latest_version));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_version_major() {
        assert!(is_newer_version("4.0.0", "3.2.0"));
        assert!(is_newer_version("2.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("3.0.0", "4.0.0"));
    }

    #[test]
    fn is_newer_version_minor() {
        assert!(is_newer_version("3.3.0", "3.2.0"));
        assert!(is_newer_version("1.2.0", "1.1.0"));
        assert!(!is_newer_version("1.1.0", "1.2.0"));
        assert!(!is_newer_version("3.2.0", "3.3.0"));
    }

    #[test]
    fn is_newer_version_patch() {
        assert!(is_newer_version("3.2.1", "3.2.0"));
        assert!(is_newer_version("1.0.5", "1.0.4"));
        assert!(!is_newer_version("1.0.4", "1.0.5"));
        assert!(!is_newer_version("3.2.0", "3.2.1"));
    }

    #[test]
    fn is_newer_version_same_version() {
        assert!(!is_newer_version("3.2.0", "3.2.0"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn is_newer_version_edge_cases() {
        // Two-part version
        assert!(is_newer_version("3.3", "3.2"));
        assert!(!is_newer_version("3.2", "3.3"));
        // One-part version
        assert!(is_newer_version("4", "3"));
        assert!(!is_newer_version("3", "4"));
        // Invalid version format
        assert!(!is_newer_version("invalid", "3.2.0"));
        assert!(!is_newer_version("3.2.0", "invalid"));
        assert!(!is_newer_version("", "3.2.0"));
    }

    #[test]
    fn should_check_for_updates_never_checked() {
        let config = Config::default();
        assert!(should_check_for_updates(&config));
    }

    #[test]
    fn should_check_for_updates_checked_recently() {
        let config = Config {
            last_update_check_timestamp: Some(current_timestamp()),
            ..Config::default()
        };
        assert!(!should_check_for_updates(&config));
    }

    #[test]
    fn should_check_for_updates_checked_long_ago() {
        let config = Config {
            last_update_check_timestamp: Some(current_timestamp() - SECONDS_PER_DAY - 1),
            ..Config::default()
        };
        assert!(should_check_for_updates(&config));
    }

    #[test]
    fn should_check_for_updates_exactly_one_day() {
        let config = Config {
            last_update_check_timestamp: Some(current_timestamp() - SECONDS_PER_DAY),
            ..Config::default()
        };
        assert!(should_check_for_updates(&config));
    }

    #[test]
    fn is_release_old_enough_handles_past_present_and_invalid() {
        // A release from far in the past should be old enough.
        assert!(is_release_old_enough("2020-01-01T00:00:00Z"));
        // A release from far in the future should not be old enough.
        assert!(!is_release_old_enough("2099-01-01T00:00:00Z"));
        // An unparsable timestamp should not be considered old enough.
        assert!(!is_release_old_enough("invalid"));
    }

    #[test]
    fn format_update_message_includes_versions_and_url() {
        let message = format_update_message("v3.3.0");
        assert!(message.contains("v3.3.0"));
        assert!(message.contains(CURRENT_VERSION));
        assert!(message.contains("https://github.com/timrogers/formanator/releases/tag/v3.3.0"));
    }
}
