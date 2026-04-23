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
pub fn handle_asset_fetch(app: &AppHandle, ipc: &IpcMessage) {
    use crate::models::ipc_message::unwrap_ipc_array;

    let Some(request_id) = ipc.request_id.clone() else {
        warn!("asset-fetch called without requestId");
        return;
    };

    let asset_path = ipc.message.as_ref()
        .map(unwrap_ipc_array)
        .and_then(|value| value.as_str())
        .unwrap_or("");

    let result = crate::services::asset_service::resolve_asset(asset_path);
    let response = IpcMessage::success(&request_id, Some(result.into_value()));
    handlers::forward_to_rendered(app, &response);
}

/// Reload both webviews — equivalent of Windows `rendered.CoreWebView2.Reload()` +
/// `background.CoreWebView2.Reload()`. Triggered by the "retry" button on the
/// unexpected error screen (`DisplayUnexpectedError` in passbolt-styleguide).
///
/// Cannot use the webview's current URL because React Router paths (e.g.
/// `/app/passwords`) are not real assets. Navigate to `index-{mode}.html` instead.
pub fn handle_reload(app: &AppHandle, state: &AppState) {
    info!("[RenderedTopic] Reloading both webviews");
    crate::services::navigation_service::navigate_to_current_mode(app, state);
}

/// Register the RBAC request for response interception, then forward to Background.
pub fn handle_rbac_request(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    if let Some(ref id) = ipc.request_id {
        state.ipc.add_pending_request(id, &ipc.topic);
    }
    handlers::forward_to_background(app, &ipc);
}
