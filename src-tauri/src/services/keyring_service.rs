//! OS keyring wrapper for persisting account credentials across sessions.
//!
//! Stores two entries in the OS keyring (macOS Keychain / Windows Credential Manager /
//! Linux Secret Service):
//! - `account-metadata`: public account info (domain, keys, security token, theme, locale)
//! - `account-secret`: armored passphrase-encrypted private key

use log::{debug, warn};

use crate::error::AppError;
use crate::models::account_metadata::{AccountMetadata, SecurityToken};
use crate::models::account_secret::AccountSecret;

const METADATA_KEY: &str = "account-metadata";
const SECRET_KEY: &str = "account-secret";

/// Keyring service for reading/writing account credentials.
pub struct KeyringService {
    service_name: String,
}

impl Default for KeyringService {
    fn default() -> Self {
        Self {
            service_name: "com.passbolt.desktop".to_string(),
        }
    }
}

impl KeyringService {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    // ── Account existence ───────────────────────────────────────

    /// Returns true if an account is stored in the keyring.
    pub fn has_account(&self) -> bool {
        self.get_account_metadata().is_some()
    }

    // ── Account metadata ────────────────────────────────────────

    /// Read account metadata from the keyring.
    pub fn get_account_metadata(&self) -> Option<AccountMetadata> {
        match self.read(METADATA_KEY) {
            Some(json) => match serde_json::from_str(&json) {
                Ok(metadata) => Some(metadata),
                Err(e) => {
                    warn!("[Keyring] Failed to parse account metadata: {}", e);
                    None
                }
            },
            None => None,
        }
    }

    /// Write account metadata to the keyring.
    pub fn save_account_metadata(&self, metadata: &AccountMetadata) -> Result<(), AppError> {
        let json = serde_json::to_string(metadata)?;
        self.write(METADATA_KEY, &json)
    }

    // ── Account secret ──────────────────────────────────────────

    /// Read account secret from the keyring.
    pub fn get_account_secret(&self) -> Option<AccountSecret> {
        match self.read(SECRET_KEY) {
            Some(json) => match serde_json::from_str(&json) {
                Ok(secret) => Some(secret),
                Err(e) => {
                    warn!("[Keyring] Failed to parse account secret: {}", e);
                    None
                }
            },
            None => None,
        }
    }

    /// Write account secret to the keyring.
    pub fn save_account_secret(&self, secret: &AccountSecret) -> Result<(), AppError> {
        let json = serde_json::to_string(secret)?;
        self.write(SECRET_KEY, &json)
    }

    // ── Bundle operations ───────────────────────────────────────

    /// Store both metadata and secret at once (used by the import flow).
    pub fn create_account(
        &self,
        metadata: &AccountMetadata,
        secret: &AccountSecret,
    ) -> Result<(), AppError> {
        self.save_account_metadata(metadata)?;
        self.save_account_secret(secret)?;
        debug!("[Keyring] Account created for {}", metadata.username);
        Ok(())
    }

    /// Remove both metadata and secret (used by import rollback).
    pub fn remove_account(&self) -> Result<(), AppError> {
        self.delete(METADATA_KEY)?;
        self.delete(SECRET_KEY)?;
        debug!("[Keyring] Account removed");
        Ok(())
    }

    // ── Field updates ───────────────────────────────────────────

    /// Update the theme field in stored metadata.
    pub fn update_theme(&self, theme: &str) -> Result<(), AppError> {
        let mut metadata = self.get_account_metadata()
            .ok_or_else(|| AppError::CredentialError("No account to update".to_string()))?;
        metadata.theme = Some(theme.to_string());
        self.save_account_metadata(&metadata)
    }

    /// Update the locale field in stored metadata.
    pub fn update_locale(&self, locale: &str) -> Result<(), AppError> {
        let mut metadata = self.get_account_metadata()
            .ok_or_else(|| AppError::CredentialError("No account to update".to_string()))?;
        metadata.locale = Some(locale.to_string());
        self.save_account_metadata(&metadata)
    }

    /// Update the security token in stored metadata.
    pub fn update_security_token(&self, token: &SecurityToken) -> Result<(), AppError> {
        let mut metadata = self.get_account_metadata()
            .ok_or_else(|| AppError::CredentialError("No account to update".to_string()))?;
        metadata.security_token = token.clone();
        self.save_account_metadata(&metadata)
    }

    /// Replace the armored private key in stored secret (key rotation).
    pub fn update_private_key(&self, armored_key: &str) -> Result<(), AppError> {
        let mut secret = self.get_account_secret()
            .ok_or_else(|| AppError::CredentialError("No account to update".to_string()))?;
        secret.user_private_armored_key = armored_key.to_string();
        self.save_account_secret(&secret)
    }

    // ── Storage backend ───────────────────────────────────────────

    // macOS/Linux: keyring crate (Keychain / Secret Service)
    #[cfg(not(target_os = "windows"))]
    fn read(&self, key: &str) -> Option<String> {
        let entry = keyring::Entry::new(&self.service_name, key).ok()?;
        entry.get_password().ok()
    }

    #[cfg(not(target_os = "windows"))]
    fn write(&self, key: &str, value: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|error| AppError::CredentialError(format!("Keyring entry error: {}", error)))?;
        entry.set_password(value)
            .map_err(|error| AppError::CredentialError(format!("Keyring write error: {}", error)))
    }

    #[cfg(not(target_os = "windows"))]
    fn delete(&self, key: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|error| AppError::CredentialError(format!("Keyring entry error: {}", error)))?;
        match entry.delete_credential() {
            Ok(_) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::CredentialError(format!("Keyring delete error: {}", error))),
        }
    }

    // Windows: PasswordVault (WinRT) — no size limit on password field,
    // unlike Win32 CredWrite which caps at 2560 bytes. Required for storing
    // armored PGP keys. Compatible with MSIX deployment.
    #[cfg(target_os = "windows")]
    fn read(&self, key: &str) -> Option<String> {
        use windows::core::HSTRING;
        use windows::Security::Credentials::PasswordVault;

        let vault = PasswordVault::new().ok()?;
        let resource = HSTRING::from(key);
        let username = HSTRING::from(&self.service_name);
        let credential = vault.Retrieve(&resource, &username).ok()?;
        credential.RetrievePassword().ok()?;
        credential.Password().ok().map(|password| password.to_string())
    }

    #[cfg(target_os = "windows")]
    fn write(&self, key: &str, value: &str) -> Result<(), AppError> {
        use windows::core::HSTRING;
        use windows::Security::Credentials::{PasswordCredential, PasswordVault};

        let vault = PasswordVault::new()
            .map_err(|error| AppError::CredentialError(format!("PasswordVault init error: {}", error)))?;

        // Remove existing credential if present (PasswordVault doesn't overwrite)
        let _ = self.delete(key);

        let credential = PasswordCredential::CreatePasswordCredential(
            &HSTRING::from(key),
            &HSTRING::from(&self.service_name),
            &HSTRING::from(value),
        )
        .map_err(|error| AppError::CredentialError(format!("PasswordVault credential error: {}", error)))?;

        vault.Add(&credential)
            .map_err(|error| AppError::CredentialError(format!("PasswordVault write error: {}", error)))
    }

    #[cfg(target_os = "windows")]
    fn delete(&self, key: &str) -> Result<(), AppError> {
        use windows::core::HSTRING;
        use windows::Security::Credentials::PasswordVault;

        let vault = PasswordVault::new()
            .map_err(|error| AppError::CredentialError(format!("PasswordVault init error: {}", error)))?;

        let resource = HSTRING::from(key);
        let username = HSTRING::from(&self.service_name);

        match vault.Retrieve(&resource, &username) {
            Ok(credential) => vault.Remove(&credential)
                .map_err(|error| AppError::CredentialError(format!("PasswordVault delete error: {}", error))),
            Err(_) => Ok(()), // Not found = already deleted
        }
    }
}
