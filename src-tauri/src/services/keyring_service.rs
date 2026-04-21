//! OS keyring wrapper for persisting account credentials across sessions.
//!
//! Stores two entries in the OS keyring (macOS Keychain / Windows Credential Manager /
//! Linux Secret Service):
//! - `account-metadata`: public account info (domain, keys, security token, theme, locale)
//! - `account-secret`: armored passphrase-encrypted private key
//!
//! In debug builds, credentials are written to local files instead of the OS keyring
//! to avoid polluting the real keychain during development.

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

impl KeyringService {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    /// Default service name for production.
    pub fn default() -> Self {
        Self::new("com.passbolt.desktop")
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

    // ── Storage backend ─────────────────────────────────────────

    #[cfg(debug_assertions)]
    fn read(&self, key: &str) -> Option<String> {
        let path = self.debug_path(key);
        std::fs::read_to_string(&path).ok()
    }

    #[cfg(debug_assertions)]
    fn write(&self, key: &str, value: &str) -> Result<(), AppError> {
        let path = self.debug_path(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::CredentialError(format!("Failed to create dir: {}", e)))?;
        }
        std::fs::write(&path, value)
            .map_err(|e| AppError::CredentialError(format!("Failed to write {}: {}", key, e)))
    }

    #[cfg(debug_assertions)]
    fn delete(&self, key: &str) -> Result<(), AppError> {
        let path = self.debug_path(key);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| AppError::CredentialError(format!("Failed to delete {}: {}", key, e)))?;
        }
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn debug_path(&self, key: &str) -> std::path::PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join(&self.service_name)
            .join("debug-credentials")
            .join(format!("{}.json", key))
    }

    #[cfg(not(debug_assertions))]
    fn read(&self, key: &str) -> Option<String> {
        let entry = keyring::Entry::new(&self.service_name, key).ok()?;
        entry.get_password().ok()
    }

    #[cfg(not(debug_assertions))]
    fn write(&self, key: &str, value: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| AppError::CredentialError(format!("Keyring entry error: {}", e)))?;
        entry.set_password(value)
            .map_err(|e| AppError::CredentialError(format!("Keyring write error: {}", e)))
    }

    #[cfg(not(debug_assertions))]
    fn delete(&self, key: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new(&self.service_name, key)
            .map_err(|e| AppError::CredentialError(format!("Keyring entry error: {}", e)))?;
        match entry.delete_credential() {
            Ok(_) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // already gone
            Err(e) => Err(AppError::CredentialError(format!("Keyring delete error: {}", e))),
        }
    }
}
