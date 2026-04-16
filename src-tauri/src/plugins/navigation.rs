use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
use log::info;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("navigation-handler")
        .on_navigation(|webview, url| {
            info!("NavigationStarting on webview '{}': {}", webview.label(), url);
            true
        })
        .on_page_load(|webview, payload| {
            let label = webview.label().to_string();
            let url = payload.url().to_string();
            let event = payload.event();

            if matches!(event, tauri::webview::PageLoadEvent::Finished) {
                info!("[Navigation] Page loaded: '{}' → {}", label, url);
            }
        })
        .on_webview_ready(|webview| {
            info!("WebviewReady: {}", webview.label());
        })
        .build()
}
