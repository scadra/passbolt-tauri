//! Account service — handles account import persistence and rollback.
//!
//! Orchestrates saving an account (metadata + secret) to the OS keyring
//! during the import flow, and rolling back on authentication failure.

use log::{info, warn};
use tauri::AppHandle;

use crate::config;
use crate::models::account_metadata::AccountMetadata;
use crate::models::account_secret::AccountSecret;
use crate::models::ipc_message::unwrap_ipc_array;
use crate::services::keyring_service::KeyringService;
use crate::services::topics::handlers;
use crate::services::webview_service::build_webview_url;
use crate::state::{AppMode, AppState};

/// Save an account from the import flow.
///
/// Parses the payload (supports nested `accountMetaData`/`accountSecret` format
/// or flat DTO), persists to the OS keyring, updates state, and navigates
/// the Background webview to auth mode.
///
/// On failure, sends an error IPC response to Rendered via `request_id`.
pub fn save_account(
    app: &AppHandle,
    state: &AppState,
    raw_payload: &serde_json::Value,
    ipc_request_id: Option<&str>,
) -> Result<(), String> {
    let payload = unwrap_ipc_array(raw_payload);

    // Try nested format (accountMetaData/accountSecret) or flat DTO
    let metadata: AccountMetadata = payload.get("accountMetaData")
        .or_else(|| payload.get("account_meta_data"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .or_else(|| serde_json::from_value(payload.clone()).ok())
        .ok_or_else(|| "Failed to parse account metadata".to_string())?;

    let secret: AccountSecret = payload.get("accountSecret")
        .or_else(|| payload.get("account_secret"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .or_else(|| serde_json::from_value(payload.clone()).ok())
        .ok_or_else(|| "Failed to parse account secret".to_string())?;

    let keyring = KeyringService::default();
    keyring.create_account(&metadata, &secret)
        .map_err(|error| format!("Failed to save account: {}", error))?;

    state.credentials.set_metadata(metadata.clone());
    state.credentials.set_secret(secret.clone());
    state.navigation.set_trusted_domain(metadata.domain.clone());
    state.navigation.set_mode(AppMode::Auth);

    // Store the requestId from the payload — will be resolved
    // when Background auth boots (pendingRequestId pattern from WinUI 3)
    if let Some(request_id) = payload.get("requestId").and_then(|value| value.as_str()) {
        state.ipc.add_pending_request(request_id, config::SAVE_ACCOUNT);
    }

    // Navigate BOTH webviews to auth with the new trusted domain.
    // Like UWP: Import → Auth reloads both webviews with the API domain.
    let bg_url = build_webview_url(
        "background-webview",
        &state.navigation.get_background_uuid(),
        &AppMode::Auth,
    );
    let rv_url = build_webview_url(
        "rendered-webview",
        &state.navigation.get_rendered_uuid(),
        &AppMode::Auth,
    );
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rv_url);

    info!("[AccountService] Account saved, navigating to auth");
    let _ = ipc_request_id; // Used by caller for error responses
    Ok(())
}

/// Roll back a failed import: remove account from keyring, reset state,
/// and navigate both webviews to import mode.
pub fn rollback_import(app: &AppHandle, state: &AppState) {
    warn!("[AccountService] Rolling back import — removing account from keyring");

    let keyring = KeyringService::default();
    if let Err(error) = keyring.remove_account() {
        warn!("[AccountService] Failed to remove account during rollback: {}", error);
    }

    state.navigation.clear_trusted_domain();
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Import);

    let bg_url = build_webview_url(
        "background-webview",
        &state.navigation.get_background_uuid(),
        &AppMode::Import,
    );
    let rv_url = build_webview_url(
        "rendered-webview",
        &state.navigation.get_rendered_uuid(),
        &AppMode::Import,
    );
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rv_url);
}
