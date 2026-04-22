//! Handler functions for Rendered webview special topics.

use log::{debug, info, warn};
use tauri::AppHandle;

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::state::AppState;
use super::handlers;

/// Rendered has mounted its listeners and is ready to receive messages.
///
/// Implements the bilateral ready handshake (same as WinUI 3):
/// when both webviews are ready, send `BACKGROUND_READY` to Rendered
/// and `RENDERED_READY` to Background. This ensures `AppAuthentication`
/// receives the signal even if Background was ready first.
pub fn handle_rendered_ready(app: &AppHandle, state: &AppState) {
    info!("[RenderedTopic] Rendered is ready");
    state.navigation.set_rendered_ready(true);

    // Drain pending messages (localStorage updates buffered during init)
    let messages = state.ipc.drain_pending_messages();
    debug!("[RenderedTopic] Draining {} pending messages", messages.len());
    for msg in messages {
        handlers::forward_to_rendered(app, &msg);
    }

    // Bilateral handshake: if both ready, notify both webviews
    if state.navigation.is_background_ready() {
        info!("[RenderedTopic] Both ready — sending bilateral handshake");
        let bg_ready = IpcMessage::new(config::BACKGROUND_READY, None);
        handlers::forward_to_rendered(app, &bg_ready);

        let rv_ready = IpcMessage::new(config::RENDERED_READY, None);
        handlers::forward_to_background(app, &rv_ready);
    }
}

/// Serve an embedded asset to the Rendered webview via IPC.
/// Used as a workaround for WKWebView blocking fetch() on custom protocol schemes.
/// Binary assets (fonts, images) are base64-encoded; text assets are sent as UTF-8.
pub fn handle_asset_fetch(app: &AppHandle, ipc: &IpcMessage) {
    use base64::Engine;
    use crate::services::protocol_service::mime_type_for_path;

    let Some(request_id) = ipc.request_id.clone() else {
        warn!("asset-fetch called without requestId");
        return;
    };

    let Some(asset_path) = extract_string_arg(&ipc.message).filter(|path| is_safe_path(path)) else {
        warn!("asset-fetch with invalid or empty path");
        let response = IpcMessage::error(&request_id, Some(serde_json::json!("Invalid asset path")));
        handlers::forward_to_rendered(app, &response);
        return;
    };

    let result = match resolve_rendered_asset(asset_path) {
        Some((file, resolved_path)) => {
            let mime = mime_type_for_path(&resolved_path);
            let is_text = mime.starts_with("text/")
                || mime == "application/json"
                || mime == "application/javascript";

            let (body, encoding) = if is_text {
                match std::str::from_utf8(&file.data) {
                    Ok(s) => (s.to_string(), "utf8"),
                    Err(_) => {
                        warn!("{} declared text but contains invalid UTF-8, falling back to base64", resolved_path);
                        (base64::engine::general_purpose::STANDARD.encode(&file.data), "base64")
                    }
                }
            } else {
                (base64::engine::general_purpose::STANDARD.encode(&file.data), "base64")
            };

            serde_json::json!({
                "status": 200,
                "body": body,
                "encoding": encoding,
                "headers": { "content-type": mime }
            })
        }
        None => {
            debug!("404 rendered asset: {}", asset_path);
            serde_json::json!({
                "status": 404,
                "body": "Not found",
                "encoding": "utf8",
                "headers": { "content-type": "text/plain" }
            })
        }
    };

    let response = IpcMessage::success(&request_id, Some(result));
    handlers::forward_to_rendered(app, &response);
}

/// Validate that a path is safe for asset resolution (no traversal, no absolute).
fn is_safe_path(path: &&str) -> bool {
    !path.is_empty() && !path.contains("..") && !path.starts_with('/')
}

/// Extract a single string argument from an IPC message payload.
/// Handles both `"value"` and `["value"]` formats.
fn extract_string_arg(message: &Option<serde_json::Value>) -> Option<&str> {
    message.as_ref().and_then(|value| {
        if let Some(array) = value.as_array() {
            array.first().and_then(|item| item.as_str())
        } else {
            value.as_str()
        }
    })
}

/// Resolve an asset path against embedded Rendered dist assets.
/// Tries exact path first, then locale fallback with `passbolt-` prefix.
fn resolve_rendered_asset(asset_path: &str) -> Option<(rust_embed::EmbeddedFile, String)> {
    use crate::services::protocol_service::RenderedDistAssets;

    if let Some(file) = RenderedDistAssets::get(asset_path) {
        return Some((file, asset_path.to_string()));
    }
    if let Some(suffix) = asset_path.strip_prefix("locales/") {
        let prefixed = format!("locales/passbolt-{}", suffix);
        if let Some(file) = RenderedDistAssets::get(&prefixed) {
            return Some((file, prefixed));
        }
    }
    None
}

/// Register the RBAC request for response interception, then forward to Background.
pub fn handle_rbac_request(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    if let Some(ref id) = ipc.request_id {
        state.ipc.add_pending_request(id, &ipc.topic);
    }
    handlers::forward_to_background(app, &ipc);
}
