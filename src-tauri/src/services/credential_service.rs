use serde::{Deserialize, Serialize};
use crate::AppResult;
use crate::error::AppError;
use crate::state::AppState;

const SERVICE_NAME: &str = "com.passbolt.desktop";

/// Mirror of C# AccountMetaData — stored as JSON in OS keyring under "account-metadata"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetaData {
    pub domain: String,
    pub user_id: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub security_token: serde_json::Value,
    pub server_public_armored_key: String,
    pub user_public_armored_key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub theme: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub locale: String,
}

/// Mirror of C# AccountSecret — stored as JSON in OS keyring under "account-secret"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSecret {
    pub user_id: String,
    pub user_private_armored_key: String,
}

pub struct CredentialService;

impl CredentialService {
    fn save(key: &str, value: &str) -> AppResult<()> {
        #[cfg(debug_assertions)]
        {
            let path = Self::debug_file_path(key);
            log::debug!("[CredentialService] Debug save to {:?}", path);
            std::fs::write(&path, value)
                .map_err(|e| AppError::Credential(format!("Failed to write '{}': {}", key, e)))?;
            return Ok(());
        }
        #[cfg(not(debug_assertions))]
        {
            let entry = keyring::Entry::new(SERVICE_NAME, key)
                .map_err(|e| AppError::Credential(format!("Failed to create entry '{}': {}", key, e)))?;
            entry
                .set_password(value)
                .map_err(|e| AppError::Credential(format!("Failed to save '{}': {}", key, e)))?;
            Ok(())
        }
    }

    fn get(key: &str) -> AppResult<String> {
        #[cfg(debug_assertions)]
        {
            let path = Self::debug_file_path(key);
            log::debug!("[CredentialService] Debug read from {:?}", path);
            return std::fs::read_to_string(&path)
                .map_err(|e| AppError::Credential(format!("Failed to read '{}': {}", key, e)));
        }
        #[cfg(not(debug_assertions))]
        {
            let entry = keyring::Entry::new(SERVICE_NAME, key)
                .map_err(|e| AppError::Credential(format!("Failed to create entry '{}': {}", key, e)))?;
            entry
                .get_password()
                .map_err(|e| AppError::Credential(format!("Failed to read '{}': {}", key, e)))
        }
    }

    fn delete(key: &str) -> AppResult<()> {
        #[cfg(debug_assertions)]
        {
            let path = Self::debug_file_path(key);
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| AppError::Credential(format!("Failed to delete '{}': {}", key, e)))?;
            }
            return Ok(());
        }
        #[cfg(not(debug_assertions))]
        {
            let entry = keyring::Entry::new(SERVICE_NAME, key)
                .map_err(|e| AppError::Credential(format!("Failed to create entry '{}': {}", key, e)))?;
            entry
                .delete_password()
                .map_err(|e| AppError::Credential(format!("Failed to delete '{}': {}", key, e)))?;
            Ok(())
        }
    }

    #[cfg(debug_assertions)]
    fn debug_file_path(key: &str) -> std::path::PathBuf {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(SERVICE_NAME)
            .join("debug-credentials");
        std::fs::create_dir_all(&dir).ok();
        dir.join(format!("{}.json", key))
    }

    /// Save account metadata + secret as two separate entries.
    pub fn create_account(metadata: &AccountMetaData, secret: &AccountSecret) -> AppResult<()> {
        Self::save("account-metadata", &serde_json::to_string(metadata)?)?;
        Self::save("account-secret", &serde_json::to_string(secret)?)?;
        log::info!("[CredentialService] Account saved for user_id={}", metadata.user_id);
        Ok(())
    }

    pub fn get_account_metadata() -> AppResult<AccountMetaData> {
        let json = Self::get("account-metadata")?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn get_account_secret() -> AppResult<AccountSecret> {
        let json = Self::get("account-secret")?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn remove_account() -> AppResult<()> {
        Self::delete("account-metadata")?;
        Self::delete("account-secret")?;
        log::info!("[CredentialService] Account removed");
        Ok(())
    }

    /// Save cached metadata from AppState to storage (no read required).
    pub fn save_cached_metadata(state: &AppState) {
        if let Some(metadata) = state.get_cached_metadata() {
            if let Err(e) = Self::save("account-metadata", &serde_json::to_string(&metadata).unwrap_or_default()) {
                log::warn!("[CredentialService] Failed to persist metadata: {}", e);
            }
        }
    }

    /// Load credentials from storage into cache (cold start only).
    pub fn load_into_cache(state: &AppState) -> AppResult<()> {
        let metadata = Self::get_account_metadata()?;
        let secret = Self::get_account_secret()?;
        state.set_cached_credentials(metadata, secret);
        Ok(())
    }
}
