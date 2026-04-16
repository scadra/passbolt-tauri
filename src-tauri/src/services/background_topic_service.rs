use tauri::{AppHandle, Emitter, Manager};

use crate::AppResult;
use crate::config;
use crate::error::AppError;
use crate::state::AppState;
use crate::models::messaging::{AllowedTopics, IPC};
use crate::services::credential_service::{CredentialService, AccountMetaData, AccountSecret};

use crate::topics::{
    auth_import_topics::AuthImportTopics,
    authentication_topics::AuthenticationTopics,
    locale_storage_topics::LocalStorageTopics,
    progress_topics::ProgressTopics,
    secret_topics::SecretTopics,
    folder_topics::FolderTopics,
    rbac_topics::RbacTopics,
};

pub struct BackgroundTopic {
    app_handle: AppHandle,
    _current_index_background: String,
    _current_index_rendered: String,
    pending_request_id: Option<String>,
    _passphrase: Option<String>,
    pending_messages: Vec<IPC>,
    rendered_ready: bool,
}

impl BackgroundTopic {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            _current_index_background: "index-auth.html".to_string(),
            _current_index_rendered: "index-auth.html".to_string(),
            pending_request_id: None,
            _passphrase: None,
            pending_messages: Vec::new(),
            rendered_ready: false,
        }
    }

    pub fn set_rendered_ready(&mut self, ready: bool) {
        self.rendered_ready = ready;
    }

    pub fn proceed_message(&mut self, ipc: IPC, state: &AppState) -> AppResult<()> {
        log::debug!("[Background → Rendered] topic={} request_id={}", ipc.topic, ipc.request_id);
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("[Background → Rendered] message={}", serde_json::to_string(&ipc).unwrap_or_default());
        }

        match ipc.topic.as_str() {
            topic if topic == AllowedTopics::BACKGROUND_READY => {
                if self.pending_request_id.is_some() {
                    self.proceed_pending_request(state)?;
                    // Navigate rendered to auth page (mirrors C# RedirectToWorkspace pattern)
                    let rendered_uuid = state.rendered_uuid();
                    if let Some(window) = self.app_handle.get_webview_window(config::RENDERED_WEBVIEW_LABEL) {
                        let url: tauri::Url = format!("{}://{}{}/app-auth",
                            config::PROTOCOL_SCHEME, rendered_uuid, config::DOMAIN_SUFFIX).parse()
                            .map_err(|e: url::ParseError| AppError::Webview(e.to_string()))?;
                        window.navigate(url)
                            .map_err(|e| AppError::Webview(e.to_string()))?;
                        log::info!("[Background] Navigating rendered to auth page");
                    }
                    // Rendered is reloading — queue background-ready for when it's ready
                    self.rendered_ready = false;
                    self.pending_messages.push(IPC::from_topic(AllowedTopics::BACKGROUND_READY.to_string()));
                } else {
                    self.post_to_rendered(&IPC::from_topic(AllowedTopics::BACKGROUND_READY.to_string()))?;
                    self.post_to_background(&IPC::from_topic(AllowedTopics::RENDERED_READY.to_string()))?;

                    if self._passphrase.is_some() {
                        let passphrase = self._passphrase.take().unwrap();
                        self.post_to_background(&IPC::from_topic_message(
                            AllowedTopics::BACKGROUND_STORE_PASSPHRASE.to_string(),
                            serde_json::Value::String(passphrase),
                        ))?;
                        // TODO: navigate rendered to index-workspace.html
                    }
                }
            }

            topic if topic == AllowedTopics::BACKGROUND_GET_COOKIE => {
                // TODO: implement CookiesManager
                log::warn!("[Background] BACKGROUND_GET_COOKIE not yet implemented");
                let response = IPC {
                    topic: ipc.request_id.clone(),
                    message: None,
                    status: Some("SUCCESS".to_string()),
                    request_id: String::new(),
                    with_multiple_param: None,
                };
                self.post_to_background(&response)?;
            }

            topic if topic == AllowedTopics::BACKGROUND_SET_THEME => {
                if let Some(ref msg) = ipc.message {
                    let value = msg.clone();
                    state.update_cached_metadata(|m| {
                        m.theme = value.as_str().unwrap_or_default().to_string();
                    });
                    CredentialService::save_cached_metadata(state);
                }
                let response = IPC {
                    topic: ipc.request_id.clone(),
                    message: None,
                    status: Some("SUCCESS".to_string()),
                    request_id: String::new(),
                    with_multiple_param: None,
                };
                self.post_to_background(&response)?;
            }

            topic if topic == AllowedTopics::BACKGROUND_SET_LOCALE => {
                if let Some(ref msg) = ipc.message {
                    let value = msg.clone();
                    state.update_cached_metadata(|m| {
                        m.locale = value.as_str().unwrap_or_default().to_string();
                    });
                    CredentialService::save_cached_metadata(state);
                }
                let response = IPC {
                    topic: ipc.request_id.clone(),
                    message: None,
                    status: Some("SUCCESS".to_string()),
                    request_id: String::new(),
                    with_multiple_param: None,
                };
                self.post_to_background(&response)?;
            }

            topic if topic == AllowedTopics::BACKGROUND_SET_SECURITY_TOKEN => {
                if let Some(ref msg) = ipc.message {
                    let value = msg.clone();
                    state.update_cached_metadata(|m| {
                        m.security_token = value;
                    });
                    CredentialService::save_cached_metadata(state);
                }
                let response = IPC {
                    topic: ipc.request_id.clone(),
                    message: None,
                    status: Some("SUCCESS".to_string()),
                    request_id: String::new(),
                    with_multiple_param: None,
                };
                self.post_to_background(&response)?;
            }

            topic if topic == AllowedTopics::BACKGROUND_ROTATE_KEY => {
                // TODO: implement CredentialLockerService - update user private key
                log::warn!("[Background] BACKGROUND_ROTATE_KEY not yet implemented");
            }

            topic if topic == AuthenticationTopics::REQUIRE_MFA => {
                // TODO: implement MFA navigation (extract passphrase, redirect rendered to MFA URL)
                log::warn!("[Background] REQUIRE_MFA not yet implemented");
            }

            topic if topic == AllowedTopics::BACKGROUND_LISTENER_READY => {
                let metadata = state.get_cached_metadata();
                let secret = state.get_cached_secret();
                if let (Some(m), Some(s)) = (metadata, secret) {
                    let account_kit = serde_json::json!({
                        "accountMetaData": m,
                        "accountSecret": s,
                    });
                    let serialized = serde_json::to_string(&account_kit).unwrap_or_default();
                    let ipc = IPC::from_topic_message(
                        "passbolt.account.set-current".to_string(),
                        serde_json::Value::String(serialized),
                    );
                    // Register the requestId so the SUCCESS response is not rejected
                    state.add_request_id(&ipc.request_id);
                    self.post_to_background(&ipc)?;
                    log::info!("[Background] Sent set-current in response to listener-ready");
                }
            }

            topic if topic == AuthImportTopics::SAVE_ACCOUNT => {
                self._current_index_background = "index-auth.html".to_string();
                self._current_index_rendered = "index-import.html".to_string();

                let raw_msg = ipc.message.clone().unwrap_or(serde_json::Value::Null);
                let msg = if raw_msg.is_array() {
                    raw_msg.as_array().and_then(|a| a.first()).cloned().unwrap_or(serde_json::Value::Null)
                } else {
                    raw_msg
                };

                self.pending_request_id = Some(ipc.request_id.clone());

                // Extract account metadata and secret from the account kit payload
                let metadata = AccountMetaData {
                    domain: msg.get("domain").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    user_id: msg.get("user_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    username: msg.get("username").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    first_name: msg.get("first_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    last_name: msg.get("last_name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    security_token: msg.get("security_token").cloned().unwrap_or(serde_json::Value::Null),
                    server_public_armored_key: msg.get("server_public_armored_key").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    user_public_armored_key: msg.get("user_public_armored_key").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    theme: msg.get("theme").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    locale: msg.get("locale").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                };

                let secret = AccountSecret {
                    user_id: msg.get("user_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    user_private_armored_key: msg.get("user_private_armored_key").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                };

                CredentialService::create_account(&metadata, &secret)?;
                state.set_cached_credentials(metadata, secret);

                let bg_uuid = state.background_uuid();
                if let Some(window) = self.app_handle.get_webview_window(config::BACKGROUND_WEBVIEW_LABEL) {
                    let url: tauri::Url = format!("{}://{}{}/background-auth",
                        config::PROTOCOL_SCHEME, bg_uuid, config::DOMAIN_SUFFIX).parse()
                        .map_err(|e: url::ParseError| AppError::Webview(e.to_string()))?;
                    window.navigate(url)
                        .map_err(|e| AppError::Webview(e.to_string()))?;
                    log::info!("[Background] Navigating to auth page");
                }
            }

            topic if topic == AllowedTopics::BACKGROUND_AUTHENTICATION_ERROR => {
                if self._current_index_rendered == "index-import.html"
                    && self._current_index_background == "index-auth.html"
                {
                    // TODO: implement - remove saved account, redirect background to import
                    log::warn!("[Background] BACKGROUND_AUTHENTICATION_ERROR not yet implemented");
                }
            }

            topic if topic == AllowedTopics::BACKGROUND_DOWNLOAD_FILE => {
                // TODO: implement DownloadService
                log::warn!("[Background] BACKGROUND_DOWNLOAD_FILE not yet implemented");
            }

            topic if topic == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_UPDATE => {
                if self.can_proceed_message(&ipc) {
                    let update_ipc = IPC::from_topic_message(
                        LocalStorageTopics::RENDERED_LOCALSTORAGE_UPDATE.to_string(),
                        serde_json::Value::String(
                            serde_json::to_string(&ipc.message).unwrap_or_default()
                        ),
                    );
                    self.post_to_rendered(&update_ipc)?;
                }
            }

            topic if topic == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_DELETE => {
                if self.can_proceed_message(&ipc) {
                    let delete_ipc = IPC::from_topic_message(
                        LocalStorageTopics::RENDERED_LOCALSTORAGE_DELETE.to_string(),
                        ipc.message.unwrap_or(serde_json::Value::Null),
                    );
                    self.post_to_rendered(&delete_ipc)?;
                }
            }

            topic if topic == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_CLEAR => {
                if self.can_proceed_message(&ipc) {
                    let clear_ipc = IPC::from_topic(LocalStorageTopics::RENDERED_LOCALSTORAGE_CLEAR.to_string());
                    self.post_to_rendered(&clear_ipc)?;
                }
            }

            topic if topic == AuthenticationTopics::LOG_OUT => {
                self._current_index_background = "index-auth.html".to_string();
                self._current_index_rendered = "index-auth.html".to_string();
                // TODO: implement - remove workspace files, recreate auth indexes, navigate both webviews
                log::warn!("[Background] LOG_OUT navigation not yet implemented");
            }

            topic if topic == AuthenticationTopics::AFTER_LOGIN => {
                self._passphrase = ipc.message.as_ref().and_then(|v| v.as_str()).map(String::from);
                // TODO: implement RedirectToWorkspace
                log::warn!("[Background] AFTER_LOGIN redirect not yet implemented");
            }

            topic if topic == SecretTopics::PASSPHRASE_REQUEST
                || topic == ProgressTopics::PROGRESSCLOSEDIALOG
                || topic == ProgressTopics::PROGRESSUPDATE
                || topic == ProgressTopics::PROGRESSUPDATEGOALS
                || topic == ProgressTopics::PROGRESSOPENDIALOG
                || topic == AllowedTopics::BACKGROUND_AFTER_LOGOUT
                || topic == FolderTopics::MOVE_STRATEGY_REQUEST =>
            {
                let request_id = &ipc.request_id;
                if !request_id.is_empty() {
                    state.add_request_id(request_id);
                }
                if self.can_proceed_message(&ipc) {
                    self.post_to_rendered(&ipc)?;
                }
            }

            topic if topic == AllowedTopics::BACKGROUND_CLIPBOARD_SET_TEXT => {
                // TODO: implement clipboard (arboard crate or tauri clipboard plugin)
                log::warn!("[Background] BACKGROUND_CLIPBOARD_SET_TEXT not yet implemented");
            }

            _ => {
                if !state.proceed_request_id(&ipc.topic) {
                    log::warn!("[Background → Rendered] REJECTED topic={}", ipc.topic);
                    return Err(AppError::UnauthorizedTopic {
                        origin: "Background webview".to_string(),
                        topic: ipc.topic.clone(),
                    });
                } else if state.has_pending_request(&ipc.topic) {
                    let value = state.get_pending_request(&ipc.topic);
                    self.map_response(&ipc, &value);
                    state.remove_pending_request(&ipc.topic);
                }
                if self.can_proceed_message(&ipc) {
                    self.post_to_rendered(&ipc)?;
                }
            }
        }

        Ok(())
    }

    pub fn proceed_pending_request(&mut self, state: &AppState) -> AppResult<()> {
        if let Some(ref pending_id) = self.pending_request_id {
            state.proceed_request_id(pending_id);

            let ipc_message = IPC {
                topic: pending_id.clone(),
                message: None,
                status: Some("SUCCESS".to_string()),
                request_id: String::new(),
                with_multiple_param: None,
            };

            self.post_to_rendered(&ipc_message)?;
            self.pending_request_id = None;
        }
        Ok(())
    }

    pub fn process_pending_messages(&mut self, state: &AppState) {
        if !self.pending_messages.is_empty() {
            let messages: Vec<IPC> = self.pending_messages.drain(..).collect();
            for msg in messages {
                if let Err(e) = self.proceed_message(msg, state) {
                    log::error!("[Background] Error processing pending message: {}", e);
                }
            }
        }
    }

    fn map_response(&self, _ipc: &IPC, topic: &str) {
        if topic == RbacTopics::FIND_ME {
            // TODO: Implement rbac service - deserialize controls, add to rbac service
        }
    }

    fn can_proceed_message(&mut self, ipc: &IPC) -> bool {
        if !self.rendered_ready && ipc.topic != AllowedTopics::BACKGROUND_READY {
            self.pending_messages.push(ipc.clone());
            return false;
        }
        true
    }

    fn post_to_rendered(&self, ipc: &IPC) -> AppResult<()> {
        self.app_handle.emit_to(config::RENDERED_WEBVIEW_LABEL, config::RENDERED_IPC_FORWARD, ipc)?;
        Ok(())
    }

    fn post_to_background(&self, ipc: &IPC) -> AppResult<()> {
        self.app_handle.emit_to(config::BACKGROUND_WEBVIEW_LABEL, config::BACKGROUND_IPC_FORWARD, ipc)?;
        Ok(())
    }
}
