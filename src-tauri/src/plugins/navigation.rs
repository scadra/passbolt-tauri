use tauri::{plugin::{Builder, TauriPlugin}, Runtime, RunEvent};

/// Creates a plugin that handles all navigation and page load events
/// 
/// This plugin provides global navigation event handling for all webviews in the application.
/// It captures navigation start, completion, and webview creation events.
/// 
/// # Returns
/// * `TauriPlugin<R>` - A configured Tauri plugin for navigation handling
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("navigation-handler")
        .on_navigation(|webview, url| {
            println!("NavigationStarting on webview '{}': {}", webview.label(), url);
            true // Allow navigation return false to avoid
        })
        .on_page_load(|webview, payload| {
            println!("NavigationCompleted on webview '{}': {} (event: {:?})", 
                webview.label(), payload.url(), payload.event());
        })
        .on_webview_ready(|webview| {
            println!("WebviewReady: {}", webview.label());
        })
        .build()
}