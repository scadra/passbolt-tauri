use std::sync::Mutex;

use super::app_mode::AppMode;

/// Webview readiness and mode tracking.
///
/// Tracks whether each webview has signaled ready, the current application mode,
/// and the trusted domain the app is locked to.
pub struct NavigationState {
    background_ready: Mutex<bool>,
    rendered_ready: Mutex<bool>,
    /// WKWebView has finished loading the page (safe to call evaluateJavaScript).
    /// Different from `*_ready` which means the JS app has mounted its listeners.
    background_page_loaded: Mutex<bool>,
    rendered_page_loaded: Mutex<bool>,
    current_mode: Mutex<AppMode>,
    trusted_domain: Mutex<Option<String>>,
    /// Tauri isolation scheme (e.g. `"isolation-870dae28-..."`) for CSP `frame-src`.
    isolation_schema: Mutex<Option<String>>,
    /// Per-webview UUIDs for unique origins in `passbolt://` URLs.
    background_uuid: Mutex<String>,
    rendered_uuid: Mutex<String>,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            background_ready: Mutex::new(false),
            rendered_ready: Mutex::new(false),
            background_page_loaded: Mutex::new(false),
            rendered_page_loaded: Mutex::new(false),
            current_mode: Mutex::new(AppMode::Import),
            trusted_domain: Mutex::new(None),
            isolation_schema: Mutex::new(None),
            background_uuid: Mutex::new(uuid::Uuid::new_v4().to_string()),
            rendered_uuid: Mutex::new(uuid::Uuid::new_v4().to_string()),
        }
    }

    // --- Ready flags ---

    pub fn set_background_ready(&self, ready: bool) {
        *self.background_ready.lock().unwrap() = ready;
    }

    pub fn is_background_ready(&self) -> bool {
        *self.background_ready.lock().unwrap()
    }

    pub fn set_rendered_ready(&self, ready: bool) {
        *self.rendered_ready.lock().unwrap() = ready;
    }

    pub fn is_rendered_ready(&self) -> bool {
        *self.rendered_ready.lock().unwrap()
    }

    // --- Page loaded flags (WKWebView safe for evaluateJavaScript) ---

    pub fn set_background_page_loaded(&self, loaded: bool) {
        *self.background_page_loaded.lock().unwrap() = loaded;
    }

    pub fn is_background_page_loaded(&self) -> bool {
        *self.background_page_loaded.lock().unwrap()
    }

    pub fn set_rendered_page_loaded(&self, loaded: bool) {
        *self.rendered_page_loaded.lock().unwrap() = loaded;
    }

    pub fn is_rendered_page_loaded(&self) -> bool {
        *self.rendered_page_loaded.lock().unwrap()
    }

    /// Reset all ready and page_loaded flags (called on mode transitions).
    pub fn reset_ready_flags(&self) {
        self.set_background_ready(false);
        self.set_rendered_ready(false);
        self.set_background_page_loaded(false);
        self.set_rendered_page_loaded(false);
    }

    // --- Mode ---

    pub fn set_mode(&self, mode: AppMode) {
        *self.current_mode.lock().unwrap() = mode;
    }

    pub fn get_mode(&self) -> AppMode {
        self.current_mode.lock().unwrap().clone()
    }

    // --- Trusted domain ---

    pub fn set_trusted_domain(&self, domain: String) {
        *self.trusted_domain.lock().unwrap() = Some(domain);
    }

    pub fn get_trusted_domain(&self) -> Option<String> {
        self.trusted_domain.lock().unwrap().clone()
    }

    pub fn clear_trusted_domain(&self) {
        *self.trusted_domain.lock().unwrap() = None;
    }

    // --- Isolation schema ---

    /// Set the Tauri isolation scheme, captured from `context.pattern()` at startup.
    pub fn set_isolation_schema(&self, schema: String) {
        *self.isolation_schema.lock().unwrap() = Some(schema);
    }

    pub fn get_isolation_schema(&self) -> Option<String> {
        self.isolation_schema.lock().unwrap().clone()
    }

    // --- Webview UUIDs ---

    pub fn get_background_uuid(&self) -> String {
        self.background_uuid.lock().unwrap().clone()
    }

    pub fn get_rendered_uuid(&self) -> String {
        self.rendered_uuid.lock().unwrap().clone()
    }
}
