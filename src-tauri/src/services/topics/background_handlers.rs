//! Handler functions for Background webview special topics.
//!
//! Each handler is a thin wrapper: extract data from `IpcMessage`,
//! delegate to the appropriate service, format/forward the response.

use log::{debug, info, warn};
use tauri::AppHandle;

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::state::AppState;
use super::handlers;

// ── Ready handshake ─────────────────────────────────────────────

/// Bilateral ready handshake (same pattern as WinUI 3):
/// when both webviews are ready, send signals to both sides.
pub fn handle_background_ready(app: &AppHandle, state: &AppState) {
    eprintln!("[MFA_PERF] handle_background_ready — Background workspace booted");
    state.navigation.set_background_ready(true);

    // Resolve pending SAVE_ACCOUNT request — send SUCCESS to Rendered so the
    // import-account Promise resolves (pendingRequestId pattern from WinUI 3).
    if let Some(pending_request_id) = state.ipc.take_pending_request_by_topic(config::SAVE_ACCOUNT) {
        let response = IpcMessage::success(&pending_request_id, None);
        handlers::forward_to_rendered(app, &response);
    }

    // Deliver cached passphrase if present (login → workspace transition).
    crate::services::auth_service::deliver_cached_passphrase(app, state);

    // Bilateral handshake: if both ready, notify both webviews
    if state.navigation.is_rendered_ready() {
        info!("[BackgroundTopic] Both ready — sending bilateral handshake");
        let bg_ready = IpcMessage::new(config::BACKGROUND_READY, None);
        handlers::forward_to_rendered(app, &bg_ready);

        let rv_ready = IpcMessage::new(config::RENDERED_READY, None);
        handlers::forward_to_background(app, &rv_ready);
    }
}

pub fn handle_listener_ready(app: &AppHandle, state: &AppState) {
    debug!("[BackgroundTopic] Background listener ready, sending account kit");
    crate::services::auth_service::send_account_kit(app, state);
}

// ── Auth flow ───────────────────────────────────────────────────

pub fn handle_after_login(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    let passphrase = ipc.message.as_ref().and_then(|msg| msg.as_str());
    crate::services::auth_service::after_login(app, state, passphrase);
}

pub fn handle_require_mfa(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    use crate::models::mfa_request::MfaRequest;

    let request: MfaRequest = ipc.message.as_ref()
        .and_then(|msg| serde_json::from_value(msg.clone()).ok())
        .unwrap_or(MfaRequest { passphrase: None, provider: "totp".to_string() });

    crate::services::auth_service::require_mfa(app, state, &request);
}

pub fn handle_logout(app: &AppHandle, state: &AppState) {
    crate::services::auth_service::logout(app, state);
}

/// Session expired — set mode to Auth, then forward the event to Rendered.
pub fn handle_after_logout(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    crate::services::auth_service::after_logout(state);
    handlers::buffer_or_forward_to_rendered(app, state, ipc);
}

// ── Import flow ─────────────────────────────────────────────────

pub fn handle_save_account(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    info!("[BackgroundTopic] Saving account from import");

    let Some(raw) = &ipc.message else {
        warn!("[BackgroundTopic] save-account: missing payload");
        return;
    };

    if let Err(error) = crate::services::account_service::save_account(
        app,
        state,
        raw,
        ipc.request_id.as_deref(),
    ) {
        warn!("[BackgroundTopic] {}", error);
        if let Some(ref request_id) = ipc.request_id {
            let response = IpcMessage::error(request_id, Some(serde_json::json!(error)));
            handlers::forward_to_rendered(app, &response);
        }
    }
}

pub fn handle_authentication_error(app: &AppHandle, state: &AppState) {
    crate::services::account_service::rollback_import(app, state);
}

// ── Vault writes ────────────────────────────────────────────────

pub fn handle_vault_write(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    debug!("[BackgroundTopic] Vault write: {}", ipc.topic);
    crate::services::settings_service::apply_vault_write(&ipc.topic, &ipc);
    handlers::buffer_or_forward_to_rendered(app, state, ipc);
}

pub fn handle_rotate_key(ipc: &IpcMessage) {
    debug!("[BackgroundTopic] Key rotation");
    if let Some(armored_key) = ipc.message.as_ref().and_then(|msg| msg.as_str()) {
        crate::services::settings_service::rotate_private_key(armored_key);
    }
}

// ── Clipboard ──────────────────────────────────────────────────

/// Write text to the system clipboard.
pub fn handle_clipboard(ipc: &IpcMessage) {
    use crate::models::clipboard_request::ClipboardRequest;
    use crate::models::ipc_message::unwrap_ipc_array;

    let payload = match ipc.message.as_ref() {
        Some(msg) => unwrap_ipc_array(msg),
        None => {
            warn!("[BackgroundTopic] Clipboard: missing payload");
            return;
        }
    };

    let request: ClipboardRequest = match serde_json::from_value(payload.clone()) {
        Ok(parsed) => parsed,
        Err(_) => {
            warn!("[BackgroundTopic] Clipboard: invalid payload");
            return;
        }
    };

    let text = request.text();
    if text.is_empty() {
        warn!("[BackgroundTopic] Clipboard: empty text");
        return;
    }

    match crate::services::clipboard_service::write_to_clipboard(text) {
        Ok(()) => info!("[BackgroundTopic] Clipboard updated ({} chars)", text.len()),
        Err(error) => warn!("[BackgroundTopic] {}", error),
    }
}

// ── Download ───────────────────────────────────────────────────

/// Download a file to ~/Downloads.
pub fn handle_download_file(ipc: &IpcMessage) {
    use crate::models::download_request::DownloadRequest;
    use crate::models::ipc_message::unwrap_ipc_array;

    let payload = match ipc.message.as_ref() {
        Some(msg) => unwrap_ipc_array(msg).clone(),
        None => {
            warn!("[BackgroundTopic] download-file: missing payload");
            return;
        }
    };

    let request: DownloadRequest = match serde_json::from_value(payload) {
        Ok(parsed) => parsed,
        Err(error) => {
            warn!("[BackgroundTopic] download-file: invalid payload: {}", error);
            return;
        }
    };

    crate::services::download_service::download(&request.content, &request.filename);
}

// ── Cookie ─────────────────────────────────────────────────────

/// Read a cookie value by name and respond to Background.
pub fn handle_get_cookie(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    use crate::models::ipc_message::unwrap_ipc_array;
    use crate::services::cookie_service;

    let cookie_name = ipc.message.as_ref()
        .map(unwrap_ipc_array)
        .and_then(|value| value.as_str())
        .unwrap_or("");

    let cookie_value = cookie_service::get_cookie(&state.cookies, cookie_name);

    if let Some(ref request_id) = ipc.request_id {
        let value = match cookie_value {
            Some(found) => serde_json::Value::String(found),
            None => serde_json::Value::Null,
        };
        let response = IpcMessage::success(request_id, Some(value));
        handlers::forward_to_background(app, &response);
    }
}

// ── HTTP fetch proxy ───────────────────────────────────────────

/// Proxy an HTTP fetch request from the Background webview through Rust.
pub fn handle_http_fetch(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    use crate::models::http_fetch_request::HttpFetchRequest;
    use crate::models::ipc_message::unwrap_ipc_array;

    let request_id = match &ipc.request_id {
        Some(id) => id.clone(),
        None => {
            warn!("[HTTP] fetch request without requestId — ignoring");
            return;
        }
    };

    let payload = match ipc.message.as_ref() {
        Some(msg) => unwrap_ipc_array(msg).clone(),
        None => {
            let response = IpcMessage::error(&request_id, Some(serde_json::json!("Missing request data")));
            handlers::forward_to_background(app, &response);
            return;
        }
    };

    let fetch_request: HttpFetchRequest = match serde_json::from_value(payload) {
        Ok(parsed) => parsed,
        Err(error) => {
            warn!("[HTTP] Invalid fetch request payload: {}", error);
            let response = IpcMessage::error(&request_id, Some(serde_json::json!("Invalid request data")));
            handlers::forward_to_background(app, &response);
            return;
        }
    };

    info!("[HTTP] {} {}", fetch_request.method, fetch_request.url);

    let trusted_domain = state.navigation.get_trusted_domain();
    let cookies = state.cookies.clone();
    let http_client = state.http_client.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        use crate::services::http_fetch_service;

        let result = http_fetch_service::handle_fetch(
            &fetch_request,
            trusted_domain.as_deref(),
            &cookies,
            &http_client,
        ).await;

        let response = IpcMessage::success(&request_id, Some(result.into_value()));
        handlers::forward_to_background(&app_clone, &response);
    });
}

// ── Response forwarding ─────────────────────────────────────────

pub fn handle_response_forward(app: &AppHandle, state: &AppState, mut ipc: IpcMessage) {
    if state.ipc.proceed_request_id(&ipc.topic) {
        if let Some(original_topic) = state.ipc.get_pending_request(&ipc.topic) {
            state.ipc.remove_pending_request(&ipc.topic);
            if original_topic == crate::topics::rbac_topics::RbacTopics::FIND_ME {
                crate::services::rbac_service::inject_desktop_denies(&mut ipc.message);
            }
        }
        handlers::forward_to_rendered(app, &ipc);
    } else {
        handlers::buffer_or_forward_to_rendered(app, state, ipc);
    }
}
