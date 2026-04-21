//! Handler functions for Rendered webview special topics.

use log::{debug, info};
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

/// Register the RBAC request for response interception, then forward to Background.
pub fn handle_rbac_request(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    if let Some(ref id) = ipc.request_id {
        state.ipc.add_pending_request(id, &ipc.topic);
    }
    handlers::forward_to_background(app, &ipc);
}
