//! Navigation guards — equivalent of MainController navigation handlers + NavigationService.
//!
//! Provides dedicated functions for each webview's navigation events, mirroring
//! the C# `MainController.RenderedNavigationStarting` / `BackgroundNavigationStarting`
//! / `AllowNavigation` / `NewWindowRequested` pattern.
//!
//! Each webview has a unique origin (`passbolt://{uuid}.passbolt.local`) and
//! may only navigate to URLs on its own origin. Cross-webview navigation is
//! blocked so a compromised script in Rendered cannot reach Background's origin
//! (where the private key lives).

use log::info;
use tauri::{AppHandle, Manager};

use crate::config;
use crate::services::mfa_service;
use crate::services::webview_service;
use crate::state::AppState;

// ── Rendered WebView ───────────────────────────────────────────

/// Rendered webview navigation guard.
/// Equivalent of `MainController.RenderedNavigationStarting()`.
///
/// Called from the `on_navigation` closure, returns `true` to allow
/// the navigation, `false` to cancel it.
pub fn rendered_navigation_starting(app: &AppHandle, url: &tauri::Url) -> bool {
    let state = app.state::<AppState>();
    let url_str = url.to_string();

    // Allow navigation to this webview's own origin
    // (equivalent of RenderedNavigationService.canNavigate checking the virtual host)
    let rendered_origin = webview_service::build_origin(&state.navigation.get_rendered_uuid());
    if url_str.starts_with(&rendered_origin) {
        return true;
    }

    // From here, URL is external (not on Rendered's origin).
    // Only allowed during MFA on the trusted domain.

    // Navigation guard (equivalent of AllowNavigation)
    if !allow_rendered_navigation(&url_str, &state) {
        // Session expiry: API redirects to /auth/login → trigger logout
        // (equivalent of: if args.Uri.EndsWith("/auth/login") → LOG_OUT)
        if url.path().ends_with("/auth/login") {
            info!("[NavigationService] Session expired — triggering logout");
            super::topics::background_handlers::handle_logout(app, &state);
        }
        return false;
    }

    // MFA completion check (equivalent of HasMfaCompleted + DisallowMfaUrls + RedirectToWorkspace)
    if state.navigation.is_mfa_active() {
        if let Some(domain) = state.navigation.get_trusted_domain() {
            if mfa_service::is_mfa_url(&url_str, &domain)
                && mfa_service::has_mfa_completed(&url_str, &domain)
            {
                info!("[NavigationService] MFA completed — redirecting to workspace");
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    handle_mfa_completed(&app_clone);
                });
                return false; // Cancel navigation (args.Cancel = true)
            }
        }
    }

    true
}

/// Check if a URL is allowed for the Rendered webview.
/// Equivalent of `RenderedNavigationService.canNavigate()`.
fn allow_rendered_navigation(url: &str, state: &AppState) -> bool {
    // During MFA: allow MFA URLs on the trusted domain
    if state.navigation.is_mfa_active() {
        if let Some(domain) = state.navigation.get_trusted_domain() {
            if mfa_service::is_mfa_url(url, &domain) {
                return true;
            }
        }
    }
    false
}

/// Post-MFA completion handler.
/// Equivalent of `DisallowMfaUrls()` + cookie extraction + `RedirectToWorkspace()`.
fn handle_mfa_completed(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Extract cookies from WKWebView → CookieState
    // (sync WKWebView's cookies back to reqwest's cookie store)
    if let Some(webview) = app.get_webview_window(config::RENDERED_WEBVIEW_LABEL) {
        if let Some(domain) = state.navigation.get_trusted_domain() {
            if let Ok(url) = domain.parse::<tauri::Url>() {
                match webview.cookies_for_url(url) {
                    Ok(cookies) => {
                        for cookie in &cookies {
                            state.cookies.set_cookie(cookie.name(), cookie.value());
                        }
                        info!("[NavigationService] Extracted {} cookies from WKWebView", cookies.len());
                    }
                    Err(error) => {
                        log::warn!("[NavigationService] Failed to extract cookies: {}", error);
                    }
                }
            }
        }
    }

    // DisallowMfaUrls equivalent
    state.navigation.set_mfa_active(false);

    // RedirectToWorkspace (passphrase already cached from handle_require_mfa)
    super::topics::background_handlers::handle_redirect_to_workspace(app, &state);
}

// ── Background WebView ─────────────────────────────────────────

/// Background webview navigation guard.
/// Equivalent of `MainController.BackgroundNavigationStarting()` + `AllowNavigation()`.
///
/// The Background webview should only ever navigate to its own origin
/// (`passbolt://{background_uuid}.passbolt.local/...`). Block everything
/// else as defense in depth — the private key lives in this webview.
pub fn background_navigation_starting(app: &AppHandle, url: &tauri::Url) -> bool {
    let state = app.state::<AppState>();
    let background_origin = webview_service::build_origin(&state.navigation.get_background_uuid());

    if url.to_string().starts_with(&background_origin) {
        return true;
    }

    log::warn!("[NavigationService] Background blocked: {}", url);
    false
}

