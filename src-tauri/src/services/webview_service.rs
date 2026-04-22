//! Webview creation and lifecycle management.

use log::info;
use tauri::{AppHandle, Manager};

use crate::config;
use crate::state::{AppMode, AppState};

/// Create both webviews with URLs matching the given mode.
///
/// Each webview gets `on_page_load` callbacks that track when WKWebView
/// has finished loading the page. This prevents `emit_to` (which calls
/// `evaluateJavaScript`) from being dispatched while the webview is
/// navigating — which would deadlock the macOS main thread.
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
    let app_for_rv = app.clone();
    let rendered_window = tauri::WebviewWindowBuilder::new(
        app,
        config::RENDERED_WEBVIEW_LABEL,
        tauri::WebviewUrl::CustomProtocol(rendered_url.parse().unwrap()),
    )
    .title("Passbolt")
    .inner_size(1024.0, 768.0)
    // Navigation guard — equivalent of MainPage.Rendered_NavigationStarting in C#.
    // Delegates to navigation_service for clear separation of concerns.
    .on_navigation(move |url| {
        super::navigation_service::rendered_navigation_starting(&app_for_rv_nav, url)
    })
    // Block all window.open / target=_blank — equivalent of NewWindowRequested in C#.
    .on_new_window(|_url, _features| {
        tauri::webview::NewWindowResponse::Deny
    })
    .on_page_load(move |_wv, payload| {
        let state = app_for_rv.state::<AppState>();
        match payload.event() {
            tauri::webview::PageLoadEvent::Started => {
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] >>> set_rv_page_loaded(false) thread={:?}", std::thread::current().id());
                }
                state.navigation.set_rendered_page_loaded(false);
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] <<< set_rv_page_loaded(false) thread={:?}", std::thread::current().id());
                    eprintln!("[PAGE] Rendered started: {}", payload.url());
                }
            }
            tauri::webview::PageLoadEvent::Finished => {
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] >>> set_rv_page_loaded(true) thread={:?}", std::thread::current().id());
                }
                state.navigation.set_rendered_page_loaded(true);
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] <<< set_rv_page_loaded(true) thread={:?}", std::thread::current().id());
                    eprintln!("[PAGE] Rendered finished: {}", payload.url());
                }
            }
        }
    })
    .build()?;

    info!("Rendered webview created (uuid: {})", rendered_uuid);

    // --- Background webview ---
    let app_for_bg_nav = app.clone();
    let app_for_bg = app.clone();
    let background_window = tauri::WebviewWindowBuilder::new(
        app,
        config::BACKGROUND_WEBVIEW_LABEL,
        tauri::WebviewUrl::CustomProtocol(background_url.parse().unwrap()),
    )
    .title("Passbolt Background")
    .visible(false)
    .skip_taskbar(true)
    // Navigation guard — equivalent of MainPage.Background_NavigationStarting in C#.
    .on_navigation(move |url| {
        super::navigation_service::background_navigation_starting(&app_for_bg_nav, url)
    })
    // Block all window.open / target=_blank.
    .on_new_window(|_url, _features| {
        tauri::webview::NewWindowResponse::Deny
    })
    .on_page_load(move |_wv, payload| {
        let state = app_for_bg.state::<AppState>();
        match payload.event() {
            tauri::webview::PageLoadEvent::Started => {
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] >>> set_bg_page_loaded(false) thread={:?}", std::thread::current().id());
                }
                state.navigation.set_background_page_loaded(false);
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] <<< set_bg_page_loaded(false) thread={:?}", std::thread::current().id());
                    eprintln!("[PAGE] Background started: {}", payload.url());
                }
            }
            tauri::webview::PageLoadEvent::Finished => {
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] >>> set_bg_page_loaded(true) thread={:?}", std::thread::current().id());
                }
                state.navigation.set_background_page_loaded(true);
                if cfg!(debug_assertions) {
                    eprintln!("[PL_LOCK] <<< set_bg_page_loaded(true) thread={:?}", std::thread::current().id());
                    eprintln!("[PAGE] Background finished: {}", payload.url());
                }
            }
        }
    })
    .build()?;

    info!("Background webview created (uuid: {})", background_uuid);

    #[cfg(debug_assertions)]
    {
        rendered_window.open_devtools();
        background_window.open_devtools();
    }

    #[cfg(not(debug_assertions))]
    {
        let _ = rendered_window;
        let _ = background_window;
    }

    Ok(())
}

/// Build the origin prefix for a webview: `passbolt://{uuid}.passbolt.local/`.
/// Used by `navigation_service` to validate that a webview only navigates
/// to URLs on its own origin.
pub fn build_origin(uuid: &str) -> String {
    format!("{}://{}{}/", config::PROTOCOL_SCHEME, uuid, config::DOMAIN_SUFFIX)
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
