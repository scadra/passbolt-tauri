//! Handler functions for Background webview special topics.

use log::{debug, info, warn};
use tauri::AppHandle;

use tauri::Manager;

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::services::keyring_service::KeyringService;
use crate::state::{AppMode, AppState};
use super::handlers;

// ── Ready handshake ─────────────────────────────────────────────

/// Bilateral ready handshake (same pattern as WinUI 3):
/// when both webviews are ready, send signals to both sides.
pub fn handle_background_ready(app: &AppHandle, state: &AppState) {
    info!("[BackgroundTopic] Background is ready");
    state.navigation.set_background_ready(true);

    // Resolve pending SAVE_ACCOUNT request — send SUCCESS to Rendered so the
    // import-account Promise resolves (pendingRequestId pattern from WinUI 3).
    if let Some(pending_request_id) = state.ipc.take_pending_request_by_topic(config::SAVE_ACCOUNT) {
        let response = IpcMessage::success(&pending_request_id, None);
        handlers::forward_to_rendered(app, &response);
    }

    // Deliver cached passphrase if present (login → workspace transition).
    // Follow WinUI 3 pattern: send passphrase THEN navigate Rendered to workspace.
    if let Some(passphrase) = state.auth.take_passphrase() {
        debug!("[BackgroundTopic] Mode transition: delivering passphrase + navigating Rendered to workspace");
        let mut msg = IpcMessage::new(
            config::BACKGROUND_STORE_PASSPHRASE,
            Some(serde_json::Value::String(passphrase)),
        );
        msg.request_id = Some(uuid::Uuid::new_v4().to_string());
        handlers::forward_to_background(app, &msg);

        // Navigate Rendered to workspace (Background already navigated by handle_redirect_to_workspace)
        handle_navigate_rendered(app, state);
    }

    // Bilateral handshake: if both ready, notify both webviews
    if state.navigation.is_rendered_ready() {
        info!("[BackgroundTopic] Both ready — sending bilateral handshake");
        let bg_ready = IpcMessage::new(config::BACKGROUND_READY, None);
        handlers::forward_to_rendered(app, &bg_ready);

        let rv_ready = IpcMessage::new(config::RENDERED_READY, None);
        handlers::forward_to_background(app, &rv_ready);
    }
    // If Rendered is not ready yet, do nothing — the bilateral handshake
    // will be triggered from handle_rendered_ready when Rendered signals ready.
}

pub fn handle_listener_ready(app: &AppHandle, state: &AppState) {
    debug!("[BackgroundTopic] Background listener ready, sending account kit");
    handle_send_account_kit(app, state);
}

// ── Auth flow ───────────────────────────────────────────────────

pub fn handle_after_login(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    info!("[BackgroundTopic] After login — caching passphrase, redirecting to workspace");
    if let Some(passphrase) = ipc.message.as_ref().and_then(|m| m.as_str()) {
        state.auth.set_passphrase(passphrase.to_string());
    }
    handle_redirect_to_workspace(app, state);
}

pub fn handle_require_mfa(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    info!("[BackgroundTopic] MFA required — caching passphrase, navigating to MFA page");
    if let Some(msg) = &ipc.message {
        // Cache passphrase for later workspace transition
        if let Some(passphrase) = msg.get("passphrase").and_then(|p| p.as_str()) {
            state.auth.set_passphrase(passphrase.to_string());
        }

        let provider = msg.get("provider")
            .and_then(|p| p.as_str())
            .unwrap_or("totp");

        // Inject cookies from CookieState into WKWebView so the server
        // recognises the session when the webview navigates to the MFA page.
        if let Some(webview) = app.get_webview_window(config::RENDERED_WEBVIEW_LABEL) {
            if let Some(domain) = state.navigation.get_trusted_domain() {
                let domain_host = domain
                    .trim_start_matches("https://")
                    .trim_start_matches("http://");
                for (name, value) in state.cookies.get_all_cookies() {
                    let base = tauri::webview::Cookie::new(&name, &value);
                    let cookie = tauri::webview::Cookie::build(base)
                        .domain(domain_host)
                        .path("/")
                        .secure(true)
                        .http_only(true)
                        .build();
                    if let Err(error) = webview.set_cookie(cookie) {
                        warn!("[BackgroundTopic] Failed to inject cookie {}: {}", name, error);
                    }
                }
            }
        }

        // AllowMfaUrls equivalent — enable MFA URL allowlist in navigation guard
        state.navigation.set_mfa_active(true);

        // Navigate Rendered WebView to the server's MFA verification page
        if let Some(domain) = state.navigation.get_trusted_domain() {
            let mfa_url = crate::services::mfa_service::build_mfa_url(&domain, provider);
            info!("[BackgroundTopic] Navigating Rendered to MFA: {}", mfa_url);
            handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &mfa_url);
        }
    }
}

pub fn handle_logout(app: &AppHandle, state: &AppState) {
    info!("[BackgroundTopic] Logout — redirecting to auth");
    state.credentials.clear();
    handle_redirect_to_auth(app, state);
}

// ── Import flow ─────────────────────────────────────────────────

pub fn handle_save_account(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    info!("[BackgroundTopic] Saving account from import");

    if let Some(raw) = &ipc.message {
        let keyring = KeyringService::default();

        // The message may be wrapped in an array by IPCHandler.request()
        let msg = if let Some(array) = raw.as_array() {
            array.first().unwrap_or(raw)
        } else {
            raw
        };

        // Try nested format (accountMetaData/accountSecret) or flat DTO
        let metadata_result = msg.get("accountMetaData")
            .or_else(|| msg.get("account_meta_data"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .or_else(|| serde_json::from_value(msg.clone()).ok());

        let secret_result = msg.get("accountSecret")
            .or_else(|| msg.get("account_secret"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .or_else(|| serde_json::from_value(msg.clone()).ok());

        if let (Some(metadata), Some(secret)) = (metadata_result, secret_result) {
            match keyring.create_account(&metadata, &secret) {
                Ok(_) => {
                    state.credentials.set_metadata(metadata.clone());
                    state.credentials.set_secret(secret.clone());
                    state.navigation.set_trusted_domain(metadata.domain.clone());
                    state.navigation.set_mode(AppMode::Auth);

                    // Store the requestId from the payload — will be resolved
                    // when Background auth boots (pendingRequestId pattern)
                    if let Some(request_id) = msg.get("requestId").and_then(|value| value.as_str()) {
                        state.ipc.add_pending_request(request_id, config::SAVE_ACCOUNT);
                    }

                    use crate::services::webview_service::build_webview_url;
                    let bg_url = build_webview_url("background-webview", &state.navigation.get_background_uuid(), &AppMode::Auth);
                    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
                }
                Err(e) => {
                    warn!("[BackgroundTopic] Failed to save account: {}", e);
                    if let Some(ref request_id) = ipc.request_id {
                        let response = IpcMessage::error(request_id, Some(serde_json::json!(format!("{}", e))));
                        handlers::forward_to_rendered(app, &response);
                    }
                }
            }
        } else {
            warn!("[BackgroundTopic] Invalid save-account payload");
        }
    }
}

pub fn handle_authentication_error(app: &AppHandle, state: &AppState) {
    warn!("[BackgroundTopic] Authentication error during import — rolling back");

    let keyring = KeyringService::default();
    if let Err(e) = keyring.remove_account() {
        warn!("[BackgroundTopic] Failed to remove account during rollback: {}", e);
    }

    state.navigation.clear_trusted_domain();
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Import);

    use crate::services::webview_service::build_webview_url;
    let bg_url = build_webview_url("background-webview", &state.navigation.get_background_uuid(), &AppMode::Import);
    let rv_url = build_webview_url("rendered-webview", &state.navigation.get_rendered_uuid(), &AppMode::Import);
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rv_url);
}

// ── Vault writes ────────────────────────────────────────────────

pub fn handle_vault_write(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    debug!("[BackgroundTopic] Vault write: {}", ipc.topic);

    let keyring = KeyringService::default();
    match ipc.topic.as_str() {
        config::BACKGROUND_SET_THEME => {
            if let Some(theme) = ipc.message.as_ref().and_then(|m| m.as_str()) {
                if let Err(e) = keyring.update_theme(theme) {
                    warn!("[BackgroundTopic] Failed to update theme: {}", e);
                }
            }
        }
        config::BACKGROUND_SET_LOCALE => {
            if let Some(locale) = ipc.message.as_ref().and_then(|m| m.as_str()) {
                if let Err(e) = keyring.update_locale(locale) {
                    warn!("[BackgroundTopic] Failed to update locale: {}", e);
                }
            }
        }
        config::BACKGROUND_SET_SECURITY_TOKEN => {
            if let Some(token) = ipc.message.as_ref().and_then(|m| serde_json::from_value(m.clone()).ok()) {
                if let Err(e) = keyring.update_security_token(&token) {
                    warn!("[BackgroundTopic] Failed to update security token: {}", e);
                }
            }
        }
        _ => {}
    }

    handlers::buffer_or_forward_to_rendered(app, state, ipc);
}

pub fn handle_rotate_key(ipc: &IpcMessage) {
    debug!("[BackgroundTopic] Key rotation");
    let keyring = KeyringService::default();
    if let Some(armored_key) = ipc.message.as_ref().and_then(|m| m.as_str()) {
        if let Err(e) = keyring.update_private_key(armored_key) {
            warn!("[BackgroundTopic] Failed to rotate key: {}", e);
        }
    }
}

// ── Utilities ───────────────────────────────────────────────────

pub fn handle_clipboard(_ipc: &IpcMessage) { debug!("[BackgroundTopic] Clipboard copy"); }
pub fn handle_download_file(_ipc: &IpcMessage) { debug!("[BackgroundTopic] File download"); }
/// Read a cookie value by name and respond to Background.
///
/// The cookie polyfill (`chrome.cookies.get`) sends a request with the cookie name.
/// We respond via the requestId so the Promise resolves.
/// If the cookie doesn't exist, we return null (like WinUI 3's CookiesManager.getCookie).
pub fn handle_get_cookie(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    let cookie_name = ipc.message.as_ref()
        .and_then(|m| {
            if let Some(arr) = m.as_array() {
                arr.first().and_then(|v| v.as_str())
            } else {
                m.as_str()
            }
        })
        .unwrap_or("");

    if cfg!(debug_assertions) {
        eprintln!("[COOKIE] get cookie_name={}", cookie_name);
    }

    // Read from the cookie store in state
    let cookie_value = state.cookies.get_cookie(cookie_name);

    if cfg!(debug_assertions) {
        eprintln!("[COOKIE] {} = {:?}", cookie_name, cookie_value);
    }

    // Respond to Background via the requestId
    if let Some(ref request_id) = ipc.request_id {
        let value = match cookie_value {
            Some(v) => serde_json::Value::String(v),
            None => serde_json::Value::Null,
        };
        let response = IpcMessage::success(request_id, Some(value));
        handlers::forward_to_background(app, &response);
    }
}

// ── HTTP fetch proxy ───────────────────────────────────────────

/// Proxy an HTTP fetch request from the Background webview through Rust.
///
/// The JS fetch polyfill intercepts `fetch('https://...')` and sends the request
/// details via IPC. We make the actual request via `reqwest`, harvest cookies,
/// and send the response back to Background via the requestId.
pub fn handle_http_fetch(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    info!("[HTTP] handle_http_fetch called — topic={} requestId={:?}", ipc.topic, ipc.request_id);

    let request_id = match &ipc.request_id {
        Some(id) => id.clone(),
        None => {
            warn!("[HTTP] fetch request without requestId — ignoring");
            return;
        }
    };

    // Extract request data (array-unwrap like handle_get_cookie)
    let request_data = ipc.message.as_ref()
        .and_then(|m| {
            if let Some(arr) = m.as_array() {
                arr.first().cloned()
            } else {
                Some(m.clone())
            }
        })
        .unwrap_or(serde_json::Value::Null);

    info!("[HTTP] request_data url={}", request_data.get("url").and_then(|v| v.as_str()).unwrap_or("NONE"));

    if request_data.is_null() {
        let response = IpcMessage::error(&request_id, Some(serde_json::json!("Missing request data")));
        handlers::forward_to_background(app, &response);
        return;
    }

    let trusted_domain = state.navigation.get_trusted_domain();
    let cookies = state.cookies.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        use crate::services::http_fetch_service;

        let result = http_fetch_service::handle_fetch(
            &request_data,
            trusted_domain.as_deref(),
            &cookies,
        ).await;

        let response = IpcMessage::success(&request_id, Some(result));
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

// ── Navigation helpers ──────────────────────────────────────────

pub fn handle_redirect_to_workspace(app: &AppHandle, state: &AppState) {
    use crate::services::webview_service::build_webview_url;
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Workspace);
    let url = build_webview_url("background-webview", &state.navigation.get_background_uuid(), &AppMode::Workspace);
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &url);
}

fn handle_redirect_to_auth(app: &AppHandle, state: &AppState) {
    use crate::services::webview_service::build_webview_url;
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Auth);
    let bg_url = build_webview_url("background-webview", &state.navigation.get_background_uuid(), &AppMode::Auth);
    let rv_url = build_webview_url("rendered-webview", &state.navigation.get_rendered_uuid(), &AppMode::Auth);
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rv_url);
}

/// Navigate Rendered to the current mode URL.
/// Spawned async to avoid blocking the IPC thread while the main thread
/// processes pending evaluateJavaScript calls from Background.
fn handle_navigate_rendered(app: &AppHandle, state: &AppState) {
    use crate::services::webview_service::build_webview_url;
    let mode = state.navigation.get_mode();
    let url = build_webview_url("rendered-webview", &state.navigation.get_rendered_uuid(), &mode);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        handlers::navigate_webview(&app, config::RENDERED_WEBVIEW_LABEL, &url);
    });
}

fn handle_send_account_kit(app: &AppHandle, state: &AppState) {
    // Use cache first, keyring as fallback
    let metadata = state.credentials.get_metadata()
        .or_else(|| {
            let keyring = KeyringService::default();
            let metadata = keyring.get_account_metadata();
            if let Some(ref metadata) = metadata {
                state.credentials.set_metadata(metadata.clone());
            }
            metadata
        });
    let secret = state.credentials.get_secret()
        .or_else(|| {
            let keyring = KeyringService::default();
            let secret = keyring.get_account_secret();
            if let Some(ref secret) = secret {
                state.credentials.set_secret(secret.clone());
            }
            secret
        });

    if let (Some(metadata), Some(secret)) = (metadata, secret) {
        let request_id = uuid::Uuid::new_v4().to_string();

        let account_kit = serde_json::json!({
            "accountMetaData": metadata,
            "accountSecret": secret
        });
        let mut msg = IpcMessage::new(config::ACCOUNT_SET_CURRENT, Some(account_kit));
        msg.request_id = Some(request_id.clone());

        state.ipc.add_request_id(&request_id);

        handlers::forward_to_background(app, &msg);
        debug!("[BackgroundTopic] Account kit sent (requestId: {})", request_id);
    } else {
        debug!("[BackgroundTopic] No account in keyring — skipping account kit");
    }
}
