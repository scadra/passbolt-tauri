//! Typed HTTP-like response envelope returned via IPC.
//!
//! Shared by `http_fetch_service` (real HTTP proxy) and `asset_service`
//! (embedded asset resolution). Both return the same `{ status, headers, body }`
//! shape expected by the JS fetch polyfill.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

/// Response envelope for IPC fetch-style requests.
///
/// Serialized to JSON and wrapped in an `IpcMessage::success()` before
/// being forwarded to the originating webview.
#[derive(Debug, Serialize)]
pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

impl FetchResponse {
    /// Create a response with full headers (used by HTTP fetch for upstream responses).
    pub fn new(status: u16, headers: HashMap<String, String>, body: String) -> Self {
        Self {
            status,
            headers,
            body,
            encoding: None,
        }
    }

    /// Create a success response with encoding and content-type header.
    /// Used by asset service for embedded file resolution.
    pub fn ok_with_encoding(body: String, content_type: &str, encoding: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), content_type.to_string());
        Self {
            status: 200,
            headers,
            body,
            encoding: Some(encoding.to_string()),
        }
    }

    /// Create a 404 not-found response.
    pub fn not_found() -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());
        Self {
            status: 404,
            headers,
            body: "Not found".to_string(),
            encoding: Some("utf8".to_string()),
        }
    }

    /// Create an error response with empty headers.
    /// Used by HTTP fetch for domain/client/request errors.
    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.to_string(),
            encoding: None,
        }
    }

    /// Create an OPTIONS preflight response with CORS headers.
    pub fn preflight() -> Self {
        let mut headers = HashMap::new();
        headers.insert(
            "access-control-allow-origin".to_string(),
            "*".to_string(),
        );
        headers.insert(
            "access-control-allow-credentials".to_string(),
            "true".to_string(),
        );
        headers.insert(
            "access-control-allow-methods".to_string(),
            "DELETE, POST, GET, OPTIONS, PUT".to_string(),
        );
        headers.insert(
            "access-control-allow-headers".to_string(),
            "Content-Type, Authorization, X-Requested-With, x-csrf-token".to_string(),
        );
        Self {
            status: 200,
            headers,
            body: String::new(),
            encoding: None,
        }
    }

    /// Convert into a `serde_json::Value` for embedding in an `IpcMessage`.
    pub fn into_value(self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "status": 500,
                "headers": {},
                "body": "Response serialization error"
            })
        })
    }
}
