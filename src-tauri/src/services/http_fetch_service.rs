//! HTTP fetch service — proxies fetch requests from the Background webview through Rust.
//!
//! On macOS, WKWebView blocks HTTP requests from custom protocol schemes (`passbolt://`).
//! The JS fetch polyfill intercepts `fetch('https://...')` calls, sends them through the
//! IPC event system, and this service makes the actual request via `reqwest`.

use std::collections::HashMap;
use serde_json::{json, Value};

use crate::state::cookie_state::CookieState;

const PWNEDPASSWORDS_URL: &str = "https://api.pwnedpasswords.com";

/// Handle a fetch request received via IPC from the Background webview.
///
/// Expects `request_data` to be a JSON object with:
/// - `url` (String): the HTTPS URL to fetch
/// - `method` (String): HTTP method (GET, POST, etc.)
/// - `headers` (Object): request headers as key-value pairs
/// - `body` (String|null): optional request body
///
/// Returns a JSON object with:
/// - `status` (u16): HTTP status code
/// - `headers` (Object): response headers
/// - `body` (String): response body as text
pub async fn handle_fetch(
    request_data: &Value,
    trusted_domain: Option<&str>,
    cookies: &CookieState,
) -> Value {
    let url = match request_data.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return error_json(400, "Missing or invalid 'url' field"),
    };

    let method_str = request_data.get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");

    let req_headers = extract_headers(request_data);

    let body = request_data.get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if cfg!(debug_assertions) {
        eprintln!("[HTTP] >>> {} {} headers={}", method_str, url, req_headers.len());
    }

    // Trusted domain check
    let is_trusted = trusted_domain
        .map(|d| url.starts_with(d))
        .unwrap_or(false);
    let is_pwnedpasswords = url.starts_with(PWNEDPASSWORDS_URL);

    if !is_trusted && !is_pwnedpasswords {
        if cfg!(debug_assertions) {
            eprintln!("[HTTP] !!! BLOCKED untrusted domain: {} (trusted={:?})", url, trusted_domain);
        }
        return error_json(403, &format!("Unauthorized domain: {}", url));
    }

    // Parse method
    let method: reqwest::Method = match method_str.parse() {
        Ok(m) => m,
        Err(_) => return error_json(400, &format!("Invalid method: {}", method_str)),
    };

    // Handle OPTIONS preflight
    if method == reqwest::Method::OPTIONS {
        if cfg!(debug_assertions) {
            eprintln!("[HTTP] <<< OPTIONS preflight → 200");
        }
        return json!({
            "status": 200,
            "headers": {
                "access-control-allow-origin": "*",
                "access-control-allow-credentials": "true",
                "access-control-allow-methods": "DELETE, POST, GET, OPTIONS, PUT",
                "access-control-allow-headers": "Content-Type, Authorization, X-Requested-With, x-csrf-token"
            },
            "body": ""
        });
    }

    // Build reqwest client (no auto-redirect)
    let client = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => return error_json(500, &format!("Client error: {}", e)),
    };

    // Build request
    let mut req_builder = client.request(method.clone(), url);

    for (key, value) in &req_headers {
        req_builder = req_builder.header(key, value);
    }

    // Inject stored cookies (session, CSRF token, etc.)
    if let Some(cookie_header) = cookies.to_cookie_header() {
        req_builder = req_builder.header("cookie", &cookie_header);
    }

    if !body.is_empty() {
        req_builder = req_builder.body(body.to_string());
    }

    // Send request
    let response = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            if cfg!(debug_assertions) {
                eprintln!("[HTTP] !!! request error: {}", e);
            }
            return error_json(502, &format!("Request failed: {}", e));
        }
    };

    let status = response.status().as_u16();

    // Harvest Set-Cookie headers
    for value in response.headers().get_all("set-cookie").iter() {
        if let Ok(cookie_str) = value.to_str() {
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
        if let Ok(v) = value.to_str() {
            resp_headers.insert(key.as_str().to_string(), v.to_string());
        }
    }

    // Read body as text
    let resp_body = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            if cfg!(debug_assertions) {
                eprintln!("[HTTP] !!! body read error: {}", e);
            }
            return error_json(502, &format!("Body read error: {}", e));
        }
    };

    if cfg!(debug_assertions) {
        let preview = &resp_body[..resp_body.len().min(100)];
        eprintln!("[HTTP] <<< {} {} → {} ({}B) body={}...", method, url, status, resp_body.len(), preview);
    }

    json!({
        "status": status,
        "headers": resp_headers,
        "body": resp_body
    })
}

fn extract_headers(request_data: &Value) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if let Some(obj) = request_data.get("headers").and_then(|v| v.as_object()) {
        for (key, value) in obj {
            if let Some(v) = value.as_str() {
                headers.insert(key.clone(), v.to_string());
            }
        }
    }
    headers
}

fn error_json(status: u16, message: &str) -> Value {
    json!({
        "status": status,
        "headers": {},
        "body": message
    })
}
