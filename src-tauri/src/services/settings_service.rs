//! Settings service — persists user preferences (theme, locale, security token, private key)
//! to the OS keyring via `KeyringService`.

use log::warn;

use crate::models::account_metadata::SecurityToken;
use crate::models::ipc_message::IpcMessage;
use crate::services::keyring_service::KeyringService;

/// Update the theme, locale, or security token based on the IPC topic.
pub fn apply_vault_write(topic: &str, ipc: &IpcMessage) {
    let keyring = KeyringService::default();

    match topic {
        crate::config::BACKGROUND_SET_THEME => {
            if let Some(theme) = ipc.message.as_ref().and_then(|msg| msg.as_str()) {
                if let Err(error) = keyring.update_theme(theme) {
                    warn!("[SettingsService] Failed to update theme: {}", error);
                }
            }
        }
        crate::config::BACKGROUND_SET_LOCALE => {
            if let Some(locale) = ipc.message.as_ref().and_then(|msg| msg.as_str()) {
                if let Err(error) = keyring.update_locale(locale) {
                    warn!("[SettingsService] Failed to update locale: {}", error);
                }
            }
        }
        crate::config::BACKGROUND_SET_SECURITY_TOKEN => {
            let token: Option<SecurityToken> = ipc.message.as_ref()
                .and_then(|msg| serde_json::from_value(msg.clone()).ok());
            if let Some(token) = token {
                if let Err(error) = keyring.update_security_token(&token) {
                    warn!("[SettingsService] Failed to update security token: {}", error);
                }
            }
        }
        _ => {}
    }
}

/// Rotate the user's private key in the OS keyring.
pub fn rotate_private_key(armored_key: &str) {
    let keyring = KeyringService::default();
    if let Err(error) = keyring.update_private_key(armored_key) {
        warn!("[SettingsService] Failed to rotate key: {}", error);
    }
}
