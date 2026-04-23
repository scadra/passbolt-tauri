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
use tauri_plugin_opener::OpenerExt;

use crate::config;
use crate::services::mfa_service;
use crate::services::topics::handlers;
use crate::services::webview_service;
use crate::state::{AppMode, AppState};

/// URL prefixes allowed to open in the system browser (prefix match).
/// Equivalent of C# `RenderedNavigationService.CanOpenBrowser()` whitelist.
const ALLOWED_EXTERNAL_PREFIXES: &[&str] = &[
    "https://help.passbolt.com/",
    "https://www.passbolt.com/",
];

/// URLs allowed to open in the system browser (exact match).
const ALLOWED_EXTERNAL_EXACT: &[&str] = &[
    "https://en.wikipedia.org/wiki/Phishing",
];

// ── Webview Navigation ────────────────────────────────────────

/// Navigate both webviews to the given mode.
///
/// Resets navigation state, sets the new mode, builds mode-specific URLs,
/// and navigates both webviews. Used by reload, redirect-to-auth, etc.
pub fn navigate_both_webviews(app: &AppHandle, state: &AppState, mode: AppMode) {
    state.reset_for_navigation();
    state.navigation.set_mode(mode.clone());
    let trusted_domain = state.navigation.get_trusted_domain();
    let background_url = webview_service::build_webview_url(
        "background-webview",
        &state.navigation.get_background_uuid(),
        &mode,
        trusted_domain.as_deref(),
    );
    let rendered_url = webview_service::build_webview_url(
        "rendered-webview",
        &state.navigation.get_rendered_uuid(),
        &mode,
        trusted_domain.as_deref(),
    );
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &background_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rendered_url);
}

/// Navigate both webviews to the current mode (reload).
///
/// Same as `navigate_both_webviews` but reads the current mode from state
/// instead of receiving it as a parameter.
pub fn navigate_to_current_mode(app: &AppHandle, state: &AppState) {
    let mode = state.navigation.get_mode();
    info!("[NavigationService] Navigating both webviews to {:?}", mode);
    navigate_both_webviews(app, state, mode);
}

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
    let trusted_domain = state.navigation.get_trusted_domain();
    let rendered_origin = webview_service::build_origin(&state.navigation.get_rendered_uuid(), trusted_domain.as_deref());
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
            crate::services::auth_service::logout(app, &state);
        }
        return false;
    }

    // During MFA, the terminal redirect ({domain}/) is ALLOWED through.
    // MFA completion is detected in on_page_load(Finished) instead of here,
    // because triggering workspace navigation from on_navigation prevents
    // the Background webview JS from executing (Tauri/WKWebView limitation).
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

/// Post-MFA completion handler, called from `on_page_load(Finished)` in
/// `webview_service.rs` when the Rendered webview finishes loading the
/// terminal redirect URL (`{domain}/`).
///
/// Called from the same execution context as on_page_load (main thread),
/// which is the same context as the normal login flow. This avoids the
/// WKWebView limitation where navigating from on_navigation's async
/// dispatch prevents subsequent JS execution.
///
/// Equivalent of C# `DisallowMfaUrls()` + cookie extraction + `RedirectToWorkspace()`.
pub fn handle_mfa_completed_from_page_load(app: &AppHandle, _state: &AppState) {
    let state = app.state::<AppState>();

    // DisallowMfaUrls equivalent
    state.navigation.set_mfa_active(false);

    // Extract cookies, then navigate Background to workspace. Rendered navigation
    // is handled by deliver_cached_passphrase() when Background signals ready.
    let app_handle = app.clone();
    let cookie_state = state.cookies.clone();
    let trusted_domain = state.navigation.get_trusted_domain();

    // Navigate Background to workspace (different webview, safe from on_page_load)
    crate::services::auth_service::redirect_to_workspace(app, &state);

    // Extract MFA cookies off the main thread (cookies_for_url deadlocks on
    // Windows main thread). Rendered navigation is NOT done here — it's handled
    // by deliver_cached_passphrase() when Background signals ready, ensuring
    // Rendered only appears after data is loaded (no empty workspace flash).
    tauri::async_runtime::spawn(async move {
        if let Some(webview) = app_handle.get_webview_window(config::RENDERED_WEBVIEW_LABEL) {
            if let Some(domain) = &trusted_domain {
                crate::services::cookie_service::extract_cookies_from_webview(
                    &webview, &cookie_state, domain,
                );
            }
        }
    });
}

// ── New Window Requests ────────────────────────────────────────

/// Handle a new window request from the Rendered webview.
/// Opens whitelisted URLs in the system browser, blocks everything else.
///
/// Equivalent of C# `MainController.NewWindowRequested()` +
/// `RenderedNavigationService.CanOpenBrowser()`.
pub fn handle_new_window_requested(app: &AppHandle, url: &str) {
    if cfg!(debug_assertions) {
        eprintln!("[NavigationService] handle_new_window_requested: url={}", url);
    }

    let is_allowed = ALLOWED_EXTERNAL_PREFIXES.iter().any(|prefix| url.starts_with(prefix))
        || ALLOWED_EXTERNAL_EXACT.contains(&url);

    if cfg!(debug_assertions) {
        eprintln!("[NavigationService] is_allowed={} for url={}", is_allowed, url);
    }

    if is_allowed {
        info!("[NavigationService] Opening whitelisted URL in browser: {}", url);
        match app.opener().open_url(url, None::<&str>) {
            Ok(_) => {
                if cfg!(debug_assertions) {
                    eprintln!("[NavigationService] opener.open_url succeeded for {}", url);
                }
            }
            Err(error) => {
                log::error!("[NavigationService] Failed to open URL: {}", error);
                if cfg!(debug_assertions) {
                    eprintln!("[NavigationService] opener.open_url FAILED: {} — url={}", error, url);
                }
            }
        }
    } else {
        info!("[NavigationService] Blocked new window request: {}", url);
        if cfg!(debug_assertions) {
            eprintln!("[NavigationService] BLOCKED (not whitelisted): {}", url);
        }
    }
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
    let trusted_domain = state.navigation.get_trusted_domain();
    let background_origin = webview_service::build_origin(&state.navigation.get_background_uuid(), trusted_domain.as_deref());
    let url_str = url.to_string();

    if url_str.starts_with(&background_origin) {
        info!("[NavigationService] Background allowed: {}", url_str);
        return true;
    }

    log::warn!("[NavigationService] Background blocked: {} (expected origin: {})", url_str, background_origin);
    false
}
