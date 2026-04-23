//! HTTP fetch service — proxies fetch requests from the Background webview through Rust.
//!
//! On macOS, WKWebView blocks HTTP requests from custom protocol schemes (`passbolt://`).
//! The JS fetch polyfill intercepts `fetch('https://...')` calls, sends them through the
//! IPC event system, and this service makes the actual request via `reqwest`.

use std::collections::HashMap;

use crate::models::fetch_response::FetchResponse;
use crate::models::http_fetch_request::HttpFetchRequest;
use crate::state::cookie_state::CookieState;

const PWNEDPASSWORDS_URL: &str = "https://api.pwnedpasswords.com";

/// Handle a fetch request received via IPC from the Background webview.
///
/// Returns a `FetchResponse` containing the HTTP status, headers, and body.
pub async fn handle_fetch(
    request: &HttpFetchRequest,
    trusted_domain: Option<&str>,
    cookies: &CookieState,
) -> FetchResponse {
    if cfg!(debug_assertions) {
        eprintln!("[HTTP] >>> {} {} headers={}", request.method, request.url, request.headers.len());
    }

    // Trusted domain check — compare parsed host+scheme, not string prefix.
    // Prevents SSRF where "https://trusted.com.attacker.com" passes a starts_with check.
    let is_trusted = trusted_domain
        .map(|domain| super::url_validation::is_trusted_origin(&request.url, domain))
        .unwrap_or(false);
    let is_pwnedpasswords = super::url_validation::is_trusted_origin(&request.url, PWNEDPASSWORDS_URL);

    if !is_trusted && !is_pwnedpasswords {
        if cfg!(debug_assertions) {
            eprintln!("[HTTP] !!! BLOCKED untrusted domain: {} (trusted={:?})", request.url, trusted_domain);
        }
        return FetchResponse::error(403, &format!("Unauthorized domain: {}", request.url));
    }

    // Parse method
    let method: reqwest::Method = match request.method.parse() {
        Ok(parsed) => parsed,
        Err(_) => return FetchResponse::error(400, &format!("Invalid method: {}", request.method)),
    };

    // Handle OPTIONS preflight
    if method == reqwest::Method::OPTIONS {
        if cfg!(debug_assertions) {
            eprintln!("[HTTP] <<< OPTIONS preflight → 200");
        }
        return FetchResponse::preflight();
    }

    // Build reqwest client — follow redirects (matches C# HttpClient default behaviour).
    // Required for MFA: GET /auth/is-authenticated.json → 302 → /mfa/verify/error.json
    let client = match reqwest::Client::builder().build() {
        Ok(built) => built,
        Err(error) => return FetchResponse::error(500, &format!("Client error: {}", error)),
    };

    // Build request
    let mut req_builder = client.request(method.clone(), &request.url);

    for (key, value) in &request.headers {
        req_builder = req_builder.header(key, value);
    }

    // Inject stored cookies (session, CSRF token, etc.)
    if let Some(cookie_header) = cookies.to_cookie_header() {
        req_builder = req_builder.header("cookie", &cookie_header);
    }

    if let Some(ref body) = request.body {
        if !body.is_empty() {
            req_builder = req_builder.body(body.clone());
        }
    }

    // Send request
    let response = match req_builder.send().await {
        Ok(resp) => resp,
        Err(error) => {
            if cfg!(debug_assertions) {
                eprintln!("[HTTP] !!! request error: {}", error);
            }
            return FetchResponse::error(502, &format!("Request failed: {}", error));
        }
    };

    let status = response.status().as_u16();

    // Harvest Set-Cookie headers
    for header_value in response.headers().get_all("set-cookie").iter() {
        if let Ok(cookie_str) = header_value.to_str() {
            if let Some(name_value) = cookie_str.split(';').next() {
                if let Some((name, val)) = name_value.split_once('=') {
                    cookies.set_cookie(name.trim(), val.trim());
                    if cfg!(debug_assertions) {
                        eprintln!("[HTTP] Cookie harvested: {}={}", name.trim(), &val.trim()[..val.len().min(20)]);
                    }
                }
            }
        }
    }

    // Collect response headers
    let mut resp_headers = HashMap::new();
    for (key, value) in response.headers().iter() {
        if let Ok(str_value) = value.to_str() {
            resp_headers.insert(key.as_str().to_string(), str_value.to_string());
        }
    }

    // Read body as text
    let resp_body = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            if cfg!(debug_assertions) {
                eprintln!("[HTTP] !!! body read error: {}", error);
            }
            return FetchResponse::error(502, &format!("Body read error: {}", error));
        }
    };

    if cfg!(debug_assertions) {
        let preview = &resp_body[..resp_body.len().min(100)];
        eprintln!("[HTTP] <<< {} {} → {} ({}B) body={}...", method, request.url, status, resp_body.len(), preview);
    }

    FetchResponse::new(status, resp_headers, resp_body)
}
