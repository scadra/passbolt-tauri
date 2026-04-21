//! Custom `passbolt://` protocol handler.
//!
//! Routes requests by host: `passbolt://{uuid}.passbolt.local/apps/{webview}/...`
//! - Index HTML requests → generated dynamically with per-mode CSP
//! - Asset requests → served from embedded `rust-embed` structs

use rust_embed::RustEmbed;
use tauri::http::{Response, StatusCode};

use crate::config;
use crate::services::local_folder_service::{self, IndexContext, WebviewTarget};
use crate::state::{AppMode, AppState};

#[derive(RustEmbed)]
#[folder = "../apps/rendered-webview/dist"]
pub struct RenderedDistAssets;

#[derive(RustEmbed)]
#[folder = "../apps/rendered-webview/src"]
pub struct RenderedSrcAssets;

#[derive(RustEmbed)]
#[folder = "../apps/background-webview/dist"]
pub struct BackgroundDistAssets;

/// Stateful protocol handler for `passbolt://` URLs.
///
/// Constructed once at startup with the isolation schema, then captured
/// by the `register_uri_scheme_protocol` closure.
pub struct ProtocolService {
    isolation_schema: Option<String>,
}

impl ProtocolService {
    pub fn new(isolation_schema: Option<String>) -> Self {
        Self { isolation_schema }
    }

    /// Handle a `passbolt://` request.
    ///
    /// Reads dynamic state (trusted_domain, UUIDs) from `AppState` at request time.
    pub fn handle_request(&self, url: &str, state: &AppState) -> Response<Vec<u8>> {
        if cfg!(debug_assertions) {
            // Use the same diag infra as handlers.rs
            use std::sync::atomic::{AtomicU64, Ordering};
            use std::sync::LazyLock;
            use std::time::Instant;
            static START: LazyLock<Instant> = LazyLock::new(Instant::now);
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let s = SEQ.fetch_add(1, Ordering::Relaxed);
            let ms = START.elapsed().as_millis();
            let short_url = if url.len() > 80 { &url[..80] } else { url };
            eprintln!("[PROTO] #{} >>> {} thread={:?} t={}ms", s, short_url, std::thread::current().id(), ms);
        }

        // Security: check for path traversal on the raw URL before parsing
        if url.contains("..") {
            return error_response(StatusCode::FORBIDDEN, "Path traversal detected");
        }

        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid URL"),
        };

        let host = parsed.host_str().unwrap_or("");
        let path = parsed.path().trim_start_matches('/');

        // Validate host ends with .passbolt.local
        if !host.ends_with(config::DOMAIN_SUFFIX) {
            return error_response(StatusCode::FORBIDDEN, "Invalid domain");
        }

        if cfg!(debug_assertions) {
            eprintln!("[LOCK] >>> get_uuids thread={:?}", std::thread::current().id());
        }
        let bg_uuid = state.navigation.get_background_uuid();
        let rv_uuid = state.navigation.get_rendered_uuid();
        let trusted_domain = state.navigation.get_trusted_domain();
        if cfg!(debug_assertions) {
            eprintln!("[LOCK] <<< get_uuids done");
        }

        // Identify which webview by UUID prefix in the host
        let (target, uuid) = if host.starts_with(&rv_uuid) {
            (WebviewTarget::Rendered, rv_uuid)
        } else if host.starts_with(&bg_uuid) {
            (WebviewTarget::Background, bg_uuid)
        } else {
            return error_response(StatusCode::NOT_FOUND, "Unknown webview origin");
        };

        let response = self.handle_webview_request(path, &target, &uuid, trusted_domain.as_deref());

        if cfg!(debug_assertions) {
            eprintln!("[PROTO] <<< status={} thread={:?}", response.status(), std::thread::current().id());
        }

        response
    }

    /// Handle a request for a specific webview.
    fn handle_webview_request(
        &self,
        path: &str,
        target: &WebviewTarget,
        uuid: &str,
        trusted_domain: Option<&str>,
    ) -> Response<Vec<u8>> {
        let webview_dir = match target {
            WebviewTarget::Rendered => "rendered-webview",
            WebviewTarget::Background => "background-webview",
        };

        // Strip the `apps/{webview}/` prefix from the path
        let prefix = format!("apps/{}/", webview_dir);
        let inner_path = path.strip_prefix(&prefix).unwrap_or(path);

        // Dynamic index HTML
        if let Some(mode) = parse_index_mode(inner_path) {
            let ctx = IndexContext {
                target,
                mode: &mode,
                uuid,
                trusted_domain,
                isolation_schema: self.isolation_schema.as_deref(),
            };
            let html = local_folder_service::generate_index_html(&ctx);
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .body(html.into_bytes())
                .unwrap();
        }

        // Static asset from dist/
        if let Some(asset_path) = inner_path.strip_prefix("dist/") {
            return serve_dist_asset(target, asset_path);
        }

        // Source files (src/js/Stylesheet.js, etc.) — Rendered only
        if let Some(src_path) = inner_path.strip_prefix("src/") {
            if *target == WebviewTarget::Rendered {
                return serve_src_asset(src_path);
            }
        }

        // Fallback: try dist
        serve_dist_asset(target, inner_path)
    }
}

/// Standalone handle function for tests that don't need the full struct.
pub fn handle_protocol_request(
    url: &str,
    _mode: &AppMode,
    trusted_domain: Option<&str>,
    isolation_schema: Option<&str>,
    background_uuid: &str,
    rendered_uuid: &str,
) -> Response<Vec<u8>> {
    if url.contains("..") {
        return error_response(StatusCode::FORBIDDEN, "Path traversal detected");
    }

    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid URL"),
    };

    let host = parsed.host_str().unwrap_or("");
    let path = parsed.path().trim_start_matches('/');

    if !host.ends_with(config::DOMAIN_SUFFIX) {
        return error_response(StatusCode::FORBIDDEN, "Invalid domain");
    }

    let (target, uuid) = if host.starts_with(rendered_uuid) {
        (WebviewTarget::Rendered, rendered_uuid)
    } else if host.starts_with(background_uuid) {
        (WebviewTarget::Background, background_uuid)
    } else {
        return error_response(StatusCode::NOT_FOUND, "Unknown webview origin");
    };

    let service = ProtocolService::new(isolation_schema.map(|s| s.to_string()));
    service.handle_webview_request(path, &target, uuid, trusted_domain)
}

fn parse_index_mode(path: &str) -> Option<AppMode> {
    match path {
        "index-import.html" => Some(AppMode::Import),
        "index-auth.html" => Some(AppMode::Auth),
        "index-workspace.html" => Some(AppMode::Workspace),
        _ => None,
    }
}

/// Serve a file from the dist/ embedded assets.
fn serve_dist_asset(target: &WebviewTarget, asset_path: &str) -> Response<Vec<u8>> {
    let data = match target {
        WebviewTarget::Rendered => RenderedDistAssets::get(asset_path),
        WebviewTarget::Background => BackgroundDistAssets::get(asset_path),
    };

    serve_embedded_file(data, asset_path)
}

/// Serve a file from the Rendered src/ embedded assets (e.g. Stylesheet.js).
fn serve_src_asset(asset_path: &str) -> Response<Vec<u8>> {
    let data = RenderedSrcAssets::get(asset_path);
    serve_embedded_file(data, asset_path)
}

fn serve_embedded_file(data: Option<rust_embed::EmbeddedFile>, asset_path: &str) -> Response<Vec<u8>> {
    match data {
        Some(file) => {
            let mime = mime_type_for_path(asset_path);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", mime)
                .header("Cache-Control", "public, max-age=31536000, immutable")
                .body(file.data.to_vec())
                .unwrap()
        }
        None => error_response(StatusCode::NOT_FOUND, "Asset not found"),
    }
}

pub fn mime_type_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("html") => "text/html; charset=utf-8",
        Some("json") => "application/json",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("map") => "application/json",
        _ => "application/octet-stream",
    }
}

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(message.as_bytes().to_vec())
        .unwrap()
}
