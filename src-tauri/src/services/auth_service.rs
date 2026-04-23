//! Auth service — orchestrates authentication lifecycle (login, MFA, logout, session expiry).
//!
//! Centralizes passphrase caching, cookie injection, mode transitions,
//! and webview navigation that were previously scattered across handlers.

use log::{debug, info};
use tauri::{AppHandle, Manager};

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::models::mfa_request::MfaRequest;
use crate::services::keyring_service::KeyringService;
use crate::services::topics::handlers;
use crate::services::webview_service::build_webview_url;
use crate::state::{AppMode, AppState};

/// Process successful login: cache passphrase and navigate to workspace.
pub fn after_login(app: &AppHandle, state: &AppState, passphrase: Option<&str>) {
    info!("[AuthService] After login — caching passphrase, redirecting to workspace");
    if let Some(passphrase) = passphrase {
        state.auth.set_passphrase(passphrase.to_string());
    }
    redirect_to_workspace(app, state);
}

/// Process MFA requirement: cache passphrase, inject cookies into WKWebView,
/// enable MFA URL allowlist, and navigate to the MFA verification page.
pub fn require_mfa(app: &AppHandle, state: &AppState, request: &MfaRequest) {
    info!("[AuthService] MFA required — provider={}", request.provider);

    // Cache passphrase for later workspace transition
    if let Some(ref passphrase) = request.passphrase {
        state.auth.set_passphrase(passphrase.as_str().to_owned());
    }

    // Inject cookies from CookieState into WKWebView so the server
    // recognises the session when the webview navigates to the MFA page.
    if let Some(webview) = app.get_webview_window(config::RENDERED_WEBVIEW_LABEL) {
        if let Some(domain) = state.navigation.get_trusted_domain() {
            crate::services::cookie_service::inject_cookies_into_webview(
                &webview,
                &state.cookies,
                &domain,
            );
        }
    }

    // AllowMfaUrls equivalent — enable MFA URL allowlist in navigation guard
    state.navigation.set_mfa_active(true);

    // Navigate Rendered WebView to the server's MFA verification page
    if let Some(domain) = state.navigation.get_trusted_domain() {
        let mfa_url = crate::services::mfa_service::build_mfa_url(&domain, &request.provider);
        info!("[AuthService] Navigating Rendered to MFA: {}", mfa_url);
        handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &mfa_url);
    }
}

/// Process voluntary logout: clear cached credentials and navigate to auth mode.
pub fn logout(app: &AppHandle, state: &AppState) {
    info!("[AuthService] Logout — redirecting to auth");
    state.credentials.clear();
    state.cookies.clear();
    redirect_to_auth(app, state);
}

/// Process session expiry: set mode to Auth so that `tab.reload` navigates
/// to auth instead of reloading the dead workspace.
pub fn after_logout(state: &AppState) {
    info!("[AuthService] Session expired — setting mode to Auth");
    state.navigation.set_mode(AppMode::Auth);
}

/// Deliver the account kit (metadata + secret) to the Background webview.
///
/// Uses the credential cache first, falling back to the OS keyring.
pub fn send_account_kit(app: &AppHandle, state: &AppState) {
    let metadata = state.credentials.get_metadata()
        .or_else(|| {
            let keyring = KeyringService::default();
            let found = keyring.get_account_metadata();
            if let Some(ref found) = found {
                state.credentials.set_metadata(found.clone());
            }
            found
        });
    let secret = state.credentials.get_secret()
        .or_else(|| {
            let keyring = KeyringService::default();
            let found = keyring.get_account_secret();
            if let Some(ref found) = found {
                state.credentials.set_secret(found.clone());
            }
            found
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
        debug!("[AuthService] Account kit sent (requestId: {})", request_id);
    } else {
        debug!("[AuthService] No account in keyring — skipping account kit");
    }
}

/// Navigate to workspace mode: reset navigation state, switch mode,
/// and navigate the Background webview. Rendered navigation is deferred
/// until Background signals ready (via `handle_background_ready`).
pub fn redirect_to_workspace(app: &AppHandle, state: &AppState) {
    info!("[AuthService] Redirecting to workspace");
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Workspace);
    let url = build_webview_url(
        "background-webview",
        &state.navigation.get_background_uuid(),
        &AppMode::Workspace,
    );
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &url);
}

/// Navigate both webviews to auth mode.
pub fn redirect_to_auth(app: &AppHandle, state: &AppState) {
    crate::services::navigation_service::navigate_both_webviews(app, state, AppMode::Auth);
}

/// Navigate Rendered to the current mode URL.
///
/// Spawned async to avoid blocking the IPC thread while the main thread
/// processes pending evaluateJavaScript calls from Background.
pub fn navigate_rendered(app: &AppHandle, state: &AppState) {
    let mode = state.navigation.get_mode();
    let url = build_webview_url(
        "rendered-webview",
        &state.navigation.get_rendered_uuid(),
        &mode,
    );
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        handlers::navigate_webview(&app, config::RENDERED_WEBVIEW_LABEL, &url);
    });
}

/// Deliver cached passphrase to Background webview during mode transition.
///
/// Called from the bilateral ready handshake when Background has booted
/// in workspace mode and a passphrase was cached during login/MFA.
pub fn deliver_cached_passphrase(app: &AppHandle, state: &AppState) {
    if let Some(passphrase) = state.auth.take_passphrase() {
        debug!("[AuthService] Delivering passphrase + navigating Rendered to workspace");
        let mut msg = IpcMessage::new(
            config::BACKGROUND_STORE_PASSPHRASE,
            Some(serde_json::Value::String(passphrase)),
        );
        msg.request_id = Some(uuid::Uuid::new_v4().to_string());
        handlers::forward_to_background(app, &msg);

        // Navigate Rendered to workspace (Background already navigated by redirect_to_workspace)
        navigate_rendered(app, state);
    }
}
