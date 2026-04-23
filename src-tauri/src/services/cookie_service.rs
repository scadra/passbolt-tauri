//! Cookie service — centralizes cookie operations between Rust state and WKWebView.
//!
//! Three operations:
//! 1. Read a cookie by name from `CookieState` (for the Background webview's cookie polyfill)
//! 2. Inject all stored cookies into a WKWebView (before MFA navigation)
//! 3. Extract cookies from a WKWebView back into `CookieState` (after MFA completion)

use log::{info, warn};
use tauri::webview::WebviewWindow;

use crate::state::cookie_state::CookieState;

/// Read a single cookie value by name from the in-memory cookie store.
pub fn get_cookie(cookies: &CookieState, name: &str) -> Option<String> {
    cookies.get_cookie(name)
}

/// Inject all stored cookies into a WKWebView for a given domain.
///
/// Strips the protocol prefix from the domain to build a bare host,
/// then sets each cookie as secure + httpOnly on the root path.
pub fn inject_cookies_into_webview(
    webview: &WebviewWindow,
    cookies: &CookieState,
    domain: &str,
) {
    let domain_host = domain
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    for (name, value) in cookies.get_all_cookies() {
        let base = tauri::webview::Cookie::new(&name, &value);
        let cookie = tauri::webview::Cookie::build(base)
            .domain(domain_host)
            .path("/")
            .secure(true)
            .http_only(true)
            .build();
        if let Err(error) = webview.set_cookie(cookie) {
            warn!("[CookieService] Failed to inject cookie {}: {}", name, error);
        }
    }
}

/// Extract cookies from a WKWebView and store them in `CookieState`.
///
/// Parses the domain as a URL, retrieves all cookies for that URL from
/// the webview, and stores each one in the in-memory cookie store.
/// Returns the number of cookies extracted.
pub fn extract_cookies_from_webview(
    webview: &WebviewWindow,
    cookie_state: &CookieState,
    domain: &str,
) -> usize {
    let url = match domain.parse::<tauri::Url>() {
        Ok(parsed) => parsed,
        Err(_) => {
            warn!("[CookieService] Failed to parse domain as URL: {}", domain);
            return 0;
        }
    };

    match webview.cookies_for_url(url) {
        Ok(cookies) => {
            let count = cookies.len();
            for cookie in &cookies {
                cookie_state.set_cookie(cookie.name(), cookie.value());
            }
            info!("[CookieService] Extracted {} cookies from WKWebView", count);
            count
        }
        Err(error) => {
            warn!("[CookieService] Failed to extract cookies: {}", error);
            0
        }
    }
}
