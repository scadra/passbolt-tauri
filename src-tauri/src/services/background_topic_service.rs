use log::{debug, warn};
use tauri::AppHandle;

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::state::AppState;
use crate::topics::authentication_topics::AuthenticationTopics;
use crate::topics::locale_storage_topics::LocalStorageTopics;
use super::topics::{handlers, background_handlers as bg};

/// Routes messages coming **from** the Background webview.
#[derive(Default)]
pub struct BackgroundTopicService;

impl BackgroundTopicService {

    pub fn handle(&self, app: &AppHandle, state: &AppState, ipc: IpcMessage) {
        debug!("[BackgroundTopic] Processing: {}", ipc.topic);

        match ipc.topic.as_str() {
            config::BACKGROUND_READY => bg::handle_background_ready(app, state),
            config::BACKGROUND_LISTENER_READY => bg::handle_listener_ready(app, state),

            t if t == AuthenticationTopics::AFTER_LOGIN => bg::handle_after_login(app, state, &ipc),
            t if t == AuthenticationTopics::REQUIRE_MFA => bg::handle_require_mfa(app, state, &ipc),
            t if t == AuthenticationTopics::LOG_OUT => bg::handle_logout(app, state),

            config::SAVE_ACCOUNT => bg::handle_save_account(app, state, &ipc),
            config::BACKGROUND_AUTHENTICATION_ERROR => bg::handle_authentication_error(app, state),

            config::BACKGROUND_SET_THEME
            | config::BACKGROUND_SET_LOCALE
            | config::BACKGROUND_SET_SECURITY_TOKEN => bg::handle_vault_write(app, state, ipc),
            config::BACKGROUND_ROTATE_KEY => bg::handle_rotate_key(&ipc),

            // Session expired: prepare auth mode, then forward dialog to Rendered.
            // The user will click "Sign in" → `tab.reload` → reloads in Auth mode.
            config::BACKGROUND_AFTER_LOGOUT => {
                bg::handle_after_logout(app, state, ipc);
            }

            // Forward to Rendered (buffered if not ready yet)
            t if t == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_UPDATE
                || t == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_DELETE
                || t == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_CLEAR =>
            {
                handlers::buffer_or_forward_to_rendered(app, state, ipc);
            }

            config::BACKGROUND_CLIPBOARD
            | config::BACKGROUND_CLIPBOARD_SET_TEXT => bg::handle_clipboard(&ipc),
            config::BACKGROUND_DOWNLOAD_FILE => bg::handle_download_file(&ipc),
            config::BACKGROUND_GET_COOKIE => bg::handle_get_cookie(app, state, &ipc),
            config::BACKGROUND_HTTP_FETCH => bg::handle_http_fetch(app, state, &ipc),

            // The catch-all handles response forwarding where the topic IS the requestId (a UUID).
            // If it doesn't look like a UUID, it's an unexpected topic — log and drop.
            unknown_topic => {
                if !super::rendered_topic_service::is_valid_uuid(unknown_topic) {
                    warn!("[BackgroundTopic] Unknown non-UUID topic, dropping: {}", unknown_topic);
                    return;
                }
                bg::handle_response_forward(app, state, ipc)
            }
        }
    }
}
