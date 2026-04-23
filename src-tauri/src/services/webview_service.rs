//! Webview creation and lifecycle management.
//!
//! Creates both webviews (Rendered + Background) and wires up:
//! - Navigation guards (`on_navigation` → `navigation_service`)
//! - New window blocking (`on_new_window` → `Deny`)
//! - Page load tracking (`on_page_load` → ready flags)
//! - Security hardening (`webview_security::harden`)
//! - DevTools (debug builds only)

use log::{info, error};
use tauri::{AppHandle, Manager};

use crate::config;
use crate::state::{AppMode, AppState};

/// Create both webviews with URLs matching the given mode.
///
/// Each webview is created hidden (`visible(false)`), hardened via
/// `webview_security::harden()`, then shown. This ensures no frame
/// loads with default (insecure) settings before hardening completes.
///
/// Equivalent of C# `MainController.LoadWebviews()` + `SetWebviewSettings()`.
pub fn create_webviews(
    app: &AppHandle,
    mode: &AppMode,
    rendered_uuid: &str,
    background_uuid: &str,
) -> Result<(), tauri::Error> {
    let rendered_url = build_webview_url("rendered-webview", rendered_uuid, mode);
    let background_url = build_webview_url("background-webview", background_uuid, mode);

    // --- Rendered webview ---
    let app_for_rv_nav = app.clone();
    let app_for_rv_new_window = app.clone();
    let app_for_rv = app.clone();
    // On Windows, navigate via the HTTP-mapped URL instead of the custom scheme.
    // The custom protocol handler is still registered and intercepts these requests.
    // This avoids WebView2's problematic custom-scheme-to-HTTP origin mapping.
    #[cfg(target_os = "windows")]
    let rendered_webview_url = tauri::WebviewUrl::External(config::to_navigation_url(&rendered_url).parse().unwrap());
    #[cfg(not(target_os = "windows"))]
    let rendered_webview_url = tauri::WebviewUrl::CustomProtocol(rendered_url.parse().unwrap());

    let rendered_window = tauri::WebviewWindowBuilder::new(
        app,
        config::RENDERED_WEBVIEW_LABEL,
        rendered_webview_url,
    )
    .title("Passbolt")
    .inner_size(1024.0, 768.0)
    .visible(false) // Hidden until hardened
    // DevTools: only in debug builds. Prevents opening inspector in release.
    .devtools(cfg!(debug_assertions))
    // Navigation guard — equivalent of MainPage.Rendered_NavigationStarting in C#.
    .on_navigation(move |url| {
        super::navigation_service::rendered_navigation_starting(&app_for_rv_nav, url)
    })
    // New window handler — equivalent of NewWindowRequested + CanOpenBrowser in C#.
    // Whitelisted URLs open in system browser; everything else is blocked.
    // The JS externalLinkPolyfill intercepts window.open() before this fires,
    // so this primarily catches <a target="_blank"> clicks.
    .on_new_window(move |url, _features| {
        if cfg!(debug_assertions) {
            eprintln!("[on_new_window] Rendered: {}", url.as_str());
        }
        super::navigation_service::handle_new_window_requested(&app_for_rv_new_window, url.as_str());
        tauri::webview::NewWindowResponse::Deny
    })
    .on_page_load(move |_wv, payload| {
        let state = app_for_rv.state::<AppState>();
        match payload.event() {
            tauri::webview::PageLoadEvent::Started => {
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] >>> set_rv_page_loaded(false) thread={:?}", std::thread::current().id());
                }
                log::info!("[PAGE] Rendered started: {}", payload.url());
                state.navigation.set_rendered_page_loaded(false);
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] <<< set_rv_page_loaded(false) thread={:?}", std::thread::current().id());
                }
            }
            tauri::webview::PageLoadEvent::Finished => {
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] >>> set_rv_page_loaded(true) thread={:?}", std::thread::current().id());
                }
                log::info!("[PAGE] Rendered finished: {}", payload.url());
                state.navigation.set_rendered_page_loaded(true);
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] <<< set_rv_page_loaded(true) thread={:?}", std::thread::current().id());
                }

                // MFA completion detection — equivalent of C# RenderedNavigationStarting
                // HasMfaCompleted check, but triggered from on_page_load instead of
                // on_navigation to ensure the workspace transition runs in the same
                // execution context as the normal login flow.
                if state.navigation.is_mfa_active() {
                    if let Some(domain) = state.navigation.get_trusted_domain() {
                        let url_str = payload.url().to_string();
                        if crate::services::mfa_service::has_mfa_completed(&url_str, &domain) {
                            log::info!("[PAGE] MFA terminal redirect detected — triggering workspace transition");
                            super::navigation_service::handle_mfa_completed_from_page_load(&app_for_rv, &state);
                        }
                    }
                }
            }
        }
    })
    .build()?;

    // Apply platform-specific security settings (swipe, zoom, link preview, etc.)
    if let Err(error) = super::webview_security::harden(&rendered_window) {
        error!("[Webview] Failed to harden rendered webview: {}", error);
    }

    // Register IPC listener scoped to this webview (source filtering).
    // This ensures only events emitted by the Rendered webview's JS are processed
    // by RenderedTopicService — equivalent of the C# canNavigate(webviewSender.Source) pattern.
    super::ipc_listener_service::register_rendered_listener(&rendered_window);

    // Show after hardening (debug: also open devtools)
    #[cfg(debug_assertions)]
    rendered_window.open_devtools();
    let _ = rendered_window.show();

    info!("Rendered webview created (uuid: {})", rendered_uuid);

    // --- Background webview ---
    let app_for_bg_nav = app.clone();
    let app_for_bg = app.clone();
    #[cfg(target_os = "windows")]
    let background_webview_url = tauri::WebviewUrl::External(config::to_navigation_url(&background_url).parse().unwrap());
    #[cfg(not(target_os = "windows"))]
    let background_webview_url = tauri::WebviewUrl::CustomProtocol(background_url.parse().unwrap());

    let background_window = tauri::WebviewWindowBuilder::new(
        app,
        config::BACKGROUND_WEBVIEW_LABEL,
        background_webview_url,
    )
    .title("Passbolt Background")
    .visible(false) // Background webview is always hidden
    .skip_taskbar(true)
    .devtools(cfg!(debug_assertions))
    // Navigation guard — equivalent of MainPage.Background_NavigationStarting in C#.
    .on_navigation(move |url| {
        super::navigation_service::background_navigation_starting(&app_for_bg_nav, url)
    })
    // Block all window.open / target=_blank.
    .on_new_window(|url, _features| {
        if cfg!(debug_assertions) {
            eprintln!("[on_new_window] Background: {}", url.as_str());
        }
        tauri::webview::NewWindowResponse::Deny
    })
    .on_page_load(move |_wv, payload| {
        let state = app_for_bg.state::<AppState>();
        match payload.event() {
            tauri::webview::PageLoadEvent::Started => {
                log::info!("[PAGE] Background started: {}", payload.url());
                state.navigation.set_background_page_loaded(false);
            }
            tauri::webview::PageLoadEvent::Finished => {
                log::info!("[PAGE] Background finished: {}", payload.url());
                state.navigation.set_background_page_loaded(true);
            }
        }
    })
    .build()?;

    // Apply platform-specific security settings
    if let Err(error) = super::webview_security::harden(&background_window) {
        error!("[Webview] Failed to harden background webview: {}", error);
    }

    // Register IPC listener scoped to this webview (source filtering).
    // This ensures only events emitted by the Background webview's JS are processed
    // by BackgroundTopicService — preventing cross-webview impersonation.
    super::ipc_listener_service::register_background_listener(&background_window);

    #[cfg(debug_assertions)]
    background_window.open_devtools();

    info!("Background webview created (uuid: {})", background_uuid);

    Ok(())
}

/// Build the browser-visible origin prefix for a webview.
///
/// macOS:   `passbolt://{uuid}.passbolt.local/`
/// Windows: `http://passbolt.{uuid}.passbolt.local/`
///
/// Used by `navigation_service` to validate that a webview only navigates
/// to URLs on its own origin.
pub fn build_origin(uuid: &str) -> String {
    config::browser_origin(uuid)
}

/// Build the `passbolt://{uuid}.passbolt.local/apps/{webview}/index-{mode}.html` URL.
pub fn build_webview_url(webview_dir: &str, uuid: &str, mode: &AppMode) -> String {
    let mode_suffix = match mode {
        AppMode::Import => "import",
        AppMode::Auth => "auth",
        AppMode::Workspace => "workspace",
    };
    format!(
        "{}://{}{}/apps/{}/index-{}.html",
        config::PROTOCOL_SCHEME,
        uuid,
        config::DOMAIN_SUFFIX,
        webview_dir,
        mode_suffix,
    )
}
