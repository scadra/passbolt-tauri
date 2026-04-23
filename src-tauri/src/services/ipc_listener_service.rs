//! Registers IPC event listeners that route messages between the two webviews.
//!
//! Each listener is scoped to a specific webview using `webview.listen()` rather
//! than `app.listen()`. This ensures the Rendered webview can only emit on
//! `rendered-ipc-message` and the Background webview can only emit on
//! `background-ipc-message` — preventing cross-webview impersonation.
//!
//! This is the Tauri v2 equivalent of the C# pattern where `WebMessageReceived`
//! identifies the sender via `webviewSender.Source` and routes accordingly:
//! ```csharp
//! if (backgroundNavigationService.canNavigate(webviewSender.Source))
//!     backgroundTopic.ProceedMessage(ipc);
//! else if (renderedNavigationService.canNavigate(webviewSender.Source))
//!     renderedTopic.ProceedMessage(ipc);
//! ```

use log::{debug, error};
use tauri::{Listener, Manager, WebviewWindow};

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::services::background_topic_service::BackgroundTopicService;
use crate::services::rendered_topic_service::RenderedTopicService;
use crate::state::AppState;

/// Register the IPC listener on the Rendered webview.
///
/// Uses `webview.listen()` so this listener ONLY receives events emitted
/// by JS running inside the Rendered webview. A compromised Background
/// webview cannot emit on `rendered-ipc-message` and have it processed here.
pub fn register_rendered_listener(webview: &WebviewWindow) {
    let rendered_service = RenderedTopicService::default();
    let app_handle = webview.app_handle().clone();

    webview.listen(config::RENDERED_IPC_MESSAGE, move |event| {
        let payload_str = event.payload();
        if let Some(result) = parse_ipc_payload(payload_str) {
            match result {
                Ok(ipc) => {
                    let state = app_handle.state::<AppState>();
                    rendered_service.handle(&app_handle, &state, ipc);
                }
                Err(parse_error) => {
                    error!("[IPC] Failed to parse rendered message: {} — payload: {}", parse_error, payload_str);
                }
            }
        }
    });

    debug!("[IPC] Rendered listener registered (scoped to webview)");
}

/// Register the IPC listener on the Background webview.
///
/// Uses `webview.listen()` so this listener ONLY receives events emitted
/// by JS running inside the Background webview. A compromised Rendered
/// webview cannot emit on `background-ipc-message` and have it processed here.
pub fn register_background_listener(webview: &WebviewWindow) {
    let background_service = BackgroundTopicService;
    let app_handle = webview.app_handle().clone();

    webview.listen(config::BACKGROUND_IPC_MESSAGE, move |event| {
        let payload_str = event.payload();
        if let Some(result) = parse_ipc_payload(payload_str) {
            match result {
                Ok(ipc) => {
                    let state = app_handle.state::<AppState>();
                    background_service.handle(&app_handle, &state, ipc);
                }
                Err(parse_error) => {
                    error!("[IPC] Failed to parse background message: {} — payload: {}", parse_error, payload_str);
                }
            }
        }
    });

    debug!("[IPC] Background listener registered (scoped to webview)");
}

/// Parse a raw JSON string into an IpcMessage.
/// Returns `None` for null/empty payloads (JS emits without data), `Some(Result)` otherwise.
pub fn parse_ipc_payload(payload: &str) -> Option<Result<IpcMessage, serde_json::Error>> {
    let trimmed = payload.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return None;
    }
    Some(serde_json::from_str::<IpcMessage>(trimmed))
}
