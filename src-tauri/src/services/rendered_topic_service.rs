use log::{debug, warn};
use tauri::AppHandle;

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::models::messaging::AllowedTopics;
use crate::state::AppState;
use crate::topics::rbac_topics::RbacTopics;
use crate::topics::folder_topics::FolderTopics;
use crate::topics::secret_topics::SecretTopics;
use crate::topics::progress_topics::ProgressTopics;
use super::topics::{handlers, rendered_handlers as rv};

/// Validates and routes messages coming **from** the Rendered webview.
pub struct RenderedTopicService {
    allowed_topics: AllowedTopics,
}

impl RenderedTopicService {
    pub fn new() -> Self {
        Self {
            allowed_topics: AllowedTopics::new(),
        }
    }

    pub fn handle(&self, app: &AppHandle, state: &AppState, ipc: IpcMessage) {
        if !self.allowed_topics.is_allowed(&ipc.topic) {
            warn!("[RenderedTopic] Unknown topic, dropping: {}", ipc.topic);
            return;
        }

        if let Some(ref id) = ipc.request_id {
            if !is_valid_uuid(id) {
                warn!("[RenderedTopic] Invalid requestId: {}", id);
                return;
            }
            state.ipc.add_request_id(id);
        }

        debug!("[RenderedTopic] Processing: {}", ipc.topic);

        match ipc.topic.as_str() {
            config::RENDERED_READY => rv::handle_rendered_ready(app, state),

            config::RENDERED_ASSET_FETCH => rv::handle_asset_fetch(app, &ipc),

            t if t == RbacTopics::FIND_ME => rv::handle_rbac_request(app, state, ipc),

            t if t == FolderTopics::MOVE_STRATEGY_REQUEST
                || t == SecretTopics::PASSPHRASE_REQUEST
                || t == ProgressTopics::PROGRESSOPENDIALOG
                || t == ProgressTopics::PROGRESSUPDATEGOALS
                || t == ProgressTopics::PROGRESSCLOSEDIALOG
                || t == ProgressTopics::PROGRESSUPDATE =>
            {
                handlers::forward_to_background(app, &ipc);
            }

            _ => handlers::forward_to_background(app, &ipc),
        }
    }
}

/// Validates that a string is a well-formed UUID (v1-5, lowercase hex).
pub fn is_valid_uuid(id: &str) -> bool {
    let re = regex_lite::Regex::new(
        r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-5][0-9a-f]{3}-[089ab][0-9a-f]{3}-[0-9a-f]{12}$"
    ).unwrap();
    re.is_match(id)
}
