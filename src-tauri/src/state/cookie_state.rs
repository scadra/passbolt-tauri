use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory cookie store — equivalent of WinUI 3's `CookiesManager`.
///
/// Uses `Arc<Mutex>` internally so it can be cheaply cloned and shared
/// across threads (needed for the async `api://` protocol handler).
///
/// Not persisted to disk — dies on app exit. Session cookies only.
#[derive(Clone)]
pub struct CookieState {
    cookies: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for CookieState {
    fn default() -> Self {
        Self {
            cookies: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl CookieState {

    /// Store a cookie name=value pair (extracted from Set-Cookie headers).
    pub fn set_cookie(&self, name: &str, value: &str) {
        self.cookies.lock().unwrap().insert(name.to_string(), value.to_string());
    }

    /// Read a cookie value by name. Returns None if not found.
    pub fn get_cookie(&self, name: &str) -> Option<String> {
        self.cookies.lock().unwrap().get(name).cloned()
    }

    /// Return all stored cookies as (name, value) pairs.
    /// Used to inject cookies into WKWebView before MFA navigation.
    pub fn get_all_cookies(&self) -> Vec<(String, String)> {
        let cookies = self.cookies.lock().unwrap();
        cookies.iter().map(|(name, value)| (name.clone(), value.clone())).collect()
    }

    /// Build a `Cookie` header string from all stored cookies.
    /// Returns None if no cookies are stored.
    pub fn to_cookie_header(&self) -> Option<String> {
        let cookies = self.cookies.lock().unwrap();
        if cookies.is_empty() {
            return None;
        }
        Some(cookies.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; "))
    }

    /// Clear all stored cookies.
    /// Called on logout to ensure session cookies don't survive across sessions.
    pub fn clear(&self) {
        self.cookies.lock().unwrap().clear();
    }
}
