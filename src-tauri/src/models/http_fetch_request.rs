use std::collections::HashMap;
use serde::Deserialize;

/// Payload for HTTP fetch proxy requests from the Background webview.
///
/// The JS fetch polyfill sends request details via IPC; Rust proxies
/// the actual request through `reqwest`.
#[derive(Debug, Deserialize)]
pub struct HttpFetchRequest {
    pub url: String,

    #[serde(default = "default_method")]
    pub method: String,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub body: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}
