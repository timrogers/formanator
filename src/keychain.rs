//! Secure token storage in the system keychain.
//!
//! Provides a cross-platform interface to store and retrieve the Forma access token
//! in the system keychain/credential manager. On macOS, this uses the Keychain.
//! On other platforms, this gracefully degrades to file-based storage.

use anyhow::Result;

#[cfg(target_os = "macos")]
const SERVICE_NAME: &str = "formanator";
#[cfg(target_os = "macos")]
const ACCOUNT_NAME: &str = "forma-access-token";

/// Store an access token in the system keychain.
///
/// On macOS, this stores the token in the Keychain.
/// On other platforms, this is a no-op to allow graceful degradation.
pub fn store_access_token(token: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)?;
        entry.set_password(token)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On non-macOS platforms, we don't store in keychain
        // The token will be stored in the config file instead
        let _ = token;
    }

    Ok(())
}

/// Retrieve an access token from the system keychain.
///
/// On macOS, this retrieves the token from the Keychain.
/// On other platforms, this returns None to allow fallback to file-based storage.
///
/// Returns `Some(token)` if a token is found, `None` if not found or not on macOS.
pub fn get_access_token() -> Result<Option<String>> {
    #[cfg(target_os = "macos")]
    {
        match keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
            Ok(entry) => match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::error::Error::NoEntry) => Ok(None),
                Err(e) => Err(anyhow::anyhow!(
                    "Failed to retrieve token from Keychain: {}",
                    e
                )),
            },
            Err(e) => {
                // If we can't create an entry object, we can't access keychain
                // Return None to allow fallback to file storage
                eprintln!("Warning: Could not access Keychain: {}", e);
                Ok(None)
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On non-macOS platforms, keychain storage is not available
        Ok(None)
    }
}

/// Remove an access token from the system keychain.
///
/// On macOS, this deletes the token from the Keychain.
/// On other platforms, this is a no-op to allow graceful degradation.
pub fn delete_access_token() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        match keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
            Ok(entry) => match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::error::Error::NoEntry) => Ok(()), // Already gone
                Err(e) => Err(anyhow::anyhow!(
                    "Failed to delete token from Keychain: {}",
                    e
                )),
            },
            Err(_) => {
                // If we can't create an entry, nothing to delete
                Ok(())
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On non-macOS platforms, there's nothing to delete
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    #[serial_test::serial]
    fn store_and_retrieve_token() {
        // Clean up before test
        let _ = delete_access_token();

        let token = "test-token-12345";
        store_access_token(token).expect("store should succeed");
        let retrieved = get_access_token().expect("get should succeed");
        assert_eq!(retrieved, Some(token.to_string()));

        // Clean up after test
        delete_access_token().expect("cleanup should succeed");
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[serial_test::serial]
    fn get_nonexistent_token_returns_none() {
        // Clean up first
        let _ = delete_access_token();

        let retrieved = get_access_token().expect("get should succeed");
        assert_eq!(retrieved, None);
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[serial_test::serial]
    fn delete_token_removes_it() {
        let token = "test-token-to-delete";
        store_access_token(token).expect("store should succeed");

        delete_access_token().expect("delete should succeed");
        let retrieved = get_access_token().expect("get should succeed");
        assert_eq!(retrieved, None);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn non_macos_store_is_noop() {
        // These should not error, just be no-ops
        store_access_token("token").expect("store should succeed");
        let retrieved = get_access_token().expect("get should succeed");
        assert_eq!(retrieved, None);
        delete_access_token().expect("delete should succeed");
    }
}
