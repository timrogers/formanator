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

/// Environment variable that, when set to a non-empty value, swaps the
/// `keyring` default credential store for an in-memory mock. This is used by
/// the integration test suite so tests do not read from or write to the
/// developer's real Keychain. It must never be set in production.
const MOCK_KEYCHAIN_ENV_VAR: &str = "FORMANATOR_USE_MOCK_KEYCHAIN";

/// Initialise keychain storage. Should be called once at process startup,
/// before any other function in this module is used.
///
/// When `FORMANATOR_USE_MOCK_KEYCHAIN` is set, swaps in `keyring`'s in-memory
/// mock credential store (see [`MOCK_KEYCHAIN_ENV_VAR`]). Otherwise this is a
/// no-op and the platform's native credential store is used.
pub fn init() {
    if std::env::var_os(MOCK_KEYCHAIN_ENV_VAR).is_some_and(|v| !v.is_empty()) {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    }
}

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
                Err(e) => {
                    // Treat any other error (e.g., no default keychain in a
                    // sandboxed environment) as "not found" so the caller can
                    // fall back to file-based storage.
                    eprintln!("Warning: Could not retrieve token from Keychain: {}", e);
                    Ok(None)
                }
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
                Err(e) => {
                    eprintln!("Warning: Could not delete token from Keychain: {}", e);
                    Ok(())
                }
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

    /// Activate the in-memory mock credential store so tests never touch the
    /// real system Keychain.
    fn use_mock_keychain() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    }

    #[test]
    #[serial_test::serial]
    fn store_and_retrieve_token() {
        use_mock_keychain();

        // Clean up before test
        let _ = delete_access_token();

        let token = "test-token-12345";
        store_access_token(token).expect("store should succeed");
        let retrieved = get_access_token().expect("get should succeed");

        #[cfg(target_os = "macos")]
        assert_eq!(retrieved, Some(token.to_string()));

        #[cfg(not(target_os = "macos"))]
        {
            // On non-macOS, keychain functions are no-ops
            assert_eq!(retrieved, None);
            let _ = token;
        }

        // Clean up after test
        delete_access_token().expect("cleanup should succeed");
    }

    #[test]
    #[serial_test::serial]
    fn get_nonexistent_token_returns_none() {
        use_mock_keychain();

        // Clean up first
        let _ = delete_access_token();

        let retrieved = get_access_token().expect("get should succeed");
        assert_eq!(retrieved, None);
    }

    #[test]
    #[serial_test::serial]
    fn delete_token_removes_it() {
        use_mock_keychain();

        let token = "test-token-to-delete";
        store_access_token(token).expect("store should succeed");

        delete_access_token().expect("delete should succeed");
        let retrieved = get_access_token().expect("get should succeed");
        assert_eq!(retrieved, None);
    }
}
