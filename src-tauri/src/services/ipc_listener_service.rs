//! Registers IPC event listeners that route messages between the two webviews.

use log::{debug, error};
use tauri::{App, Listener, Manager};

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::services::background_topic_service::BackgroundTopicService;
use crate::services::rendered_topic_service::RenderedTopicService;
use crate::state::AppState;

/// Wire up IPC event listeners.
pub fn setup(app: &App) {
    let rendered_service = RenderedTopicService::default();
    let background_service = BackgroundTopicService;

    let app_handle = app.handle().clone();
    app.listen(config::RENDERED_IPC_MESSAGE, move |event| {
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

    let app_handle = app.handle().clone();
    app.listen(config::BACKGROUND_IPC_MESSAGE, move |event| {
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

    debug!("[IPC] Listeners registered");
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
