//! Asset service — resolves and serves embedded Rendered webview assets via IPC.
//!
//! WKWebView blocks `fetch()` on custom protocol schemes (`passbolt://`).
//! The Rendered webview requests assets through IPC instead, and this service
//! resolves them from the embedded `RenderedDistAssets`.

use log::{debug, warn};

use crate::models::fetch_response::FetchResponse;
use crate::services::protocol_service::{mime_type_for_path, RenderedDistAssets};

/// Resolve an asset path and return a typed `FetchResponse`.
///
/// Returns a response matching the `{ status, body, encoding, headers }` shape
/// expected by the Rendered webview's fetch polyfill.
/// Binary assets are base64-encoded; text assets are sent as UTF-8 strings.
pub fn resolve_asset(asset_path: &str) -> FetchResponse {
    use base64::Engine;

    if !is_safe_path(asset_path) {
        warn!("[AssetService] Unsafe path rejected: {}", asset_path);
        return FetchResponse::not_found();
    }

    match resolve_rendered_asset(asset_path) {
        Some((file, resolved_path)) => {
            let mime = mime_type_for_path(&resolved_path);
            let is_text = mime.starts_with("text/")
                || mime == "application/json"
                || mime == "application/javascript";

            let (body, encoding) = if is_text {
                match std::str::from_utf8(&file.data) {
                    Ok(text) => (text.to_string(), "utf8"),
                    Err(_) => {
                        warn!("[AssetService] {} declared text but invalid UTF-8, falling back to base64", resolved_path);
                        (base64::engine::general_purpose::STANDARD.encode(&file.data), "base64")
                    }
                }
            } else {
                (base64::engine::general_purpose::STANDARD.encode(&file.data), "base64")
            };

            FetchResponse::ok_with_encoding(body, mime, encoding)
        }
        None => {
            debug!("[AssetService] 404: {}", asset_path);
            FetchResponse::not_found()
        }
    }
}

/// Validate that a path is safe for asset resolution (no traversal, no absolute).
fn is_safe_path(path: &str) -> bool {
    !path.is_empty() && !path.contains("..") && !path.starts_with('/')
}

/// Resolve an asset path against embedded Rendered dist assets.
/// Tries exact path first, then locale fallback with `passbolt-` prefix.
fn resolve_rendered_asset(asset_path: &str) -> Option<(rust_embed::EmbeddedFile, String)> {
    if let Some(file) = RenderedDistAssets::get(asset_path) {
        return Some((file, asset_path.to_string()));
    }
    if let Some(suffix) = asset_path.strip_prefix("locales/") {
        let prefixed = format!("locales/passbolt-{}", suffix);
        if let Some(file) = RenderedDistAssets::get(&prefixed) {
            return Some((file, prefixed));
        }
    }
    None
}

