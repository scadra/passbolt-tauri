use passbolt::services::protocol_service::{self, mime_type_for_path};
use passbolt::state::AppMode;

const BG_UUID: &str = "bg-uuid-1234-5678-9abc-def012345678";
const RV_UUID: &str = "rv-uuid-1234-5678-9abc-def012345678";

fn request(path: &str, mode: &AppMode, trusted_domain: Option<&str>) -> tauri::http::Response<Vec<u8>> {
    protocol_service::handle_protocol_request(path, mode, trusted_domain, None, BG_UUID, RV_UUID)
}

// ── Routing ─────────────────────────────────────────────────────

/// Rendered index URL returns dynamically generated HTML.
#[test]
fn routes_rendered_index_to_dynamic_html() {
    let url = format!("passbolt://{}.passbolt.local/apps/rendered-webview/index-auth.html", RV_UUID);
    let response = request(&url, &AppMode::Auth, Some("https://passbolt.example.com"));
    assert_eq!(response.status(), 200);
    let body = String::from_utf8(response.body().clone()).unwrap();
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("Content-Security-Policy"));
    assert!(body.contains("rendered-auth.js"));
}

/// Background index URL returns dynamically generated HTML.
#[test]
fn routes_background_index_to_dynamic_html() {
    let url = format!("passbolt://{}.passbolt.local/apps/background-webview/index-import.html", BG_UUID);
    let response = request(&url, &AppMode::Import, None);
    assert_eq!(response.status(), 200);
    let body = String::from_utf8(response.body().clone()).unwrap();
    assert!(body.contains("background-import.js"));
}

/// Asset URL serves the embedded JS bundle.
#[test]
fn routes_rendered_dist_to_embedded_asset() {
    let url = format!("passbolt://{}.passbolt.local/apps/rendered-webview/dist/rendered-auth.js", RV_UUID);
    let response = request(&url, &AppMode::Auth, None);
    assert_eq!(response.status(), 200);
    let content_type = response.headers().get("Content-Type").unwrap().to_str().unwrap();
    assert_eq!(content_type, "application/javascript");
}

/// Background asset URL serves the embedded JS bundle.
#[test]
fn routes_background_dist_to_embedded_asset() {
    let url = format!("passbolt://{}.passbolt.local/apps/background-webview/dist/background-auth.js", BG_UUID);
    let response = request(&url, &AppMode::Auth, None);
    assert_eq!(response.status(), 200);
}

// ── MIME types ──────────────────────────────────────────────────

#[test]
fn js_file_has_javascript_mime_type() {
    assert_eq!(mime_type_for_path("rendered-auth.js"), "application/javascript");
}

#[test]
fn css_file_has_css_mime_type() {
    assert_eq!(mime_type_for_path("themes/default/style.css"), "text/css");
}

#[test]
fn json_file_has_json_mime_type() {
    assert_eq!(mime_type_for_path("locales/en/common.json"), "application/json");
}

#[test]
fn html_has_html_mime_type() {
    assert_eq!(mime_type_for_path("index.html"), "text/html; charset=utf-8");
}

#[test]
fn woff2_has_font_mime_type() {
    assert_eq!(mime_type_for_path("fonts/open-sans.woff2"), "font/woff2");
}

#[test]
fn unknown_extension_returns_octet_stream() {
    assert_eq!(mime_type_for_path("file.xyz"), "application/octet-stream");
}

// ── Security ────────────────────────────────────────────────────

/// Path traversal attempts must return 403 Forbidden.
#[test]
fn path_traversal_returns_forbidden() {
    let url = format!("passbolt://{}.passbolt.local/apps/rendered-webview/dist/../../../etc/passwd", RV_UUID);
    let response = request(&url, &AppMode::Import, None);
    assert_eq!(response.status(), 403);
}

/// Unknown host (not matching any UUID) returns 404.
#[test]
fn unknown_uuid_returns_not_found() {
    let response = request(
        "passbolt://evil-uuid.passbolt.local/apps/rendered-webview/dist/file.js",
        &AppMode::Import,
        None,
    );
    assert_eq!(response.status(), 404);
}

/// Host not ending in .passbolt.local returns 403.
#[test]
fn invalid_domain_returns_forbidden() {
    let response = request("passbolt://evil.com/file.js", &AppMode::Import, None);
    assert_eq!(response.status(), 403);
}

/// Missing file returns 404.
#[test]
fn missing_file_returns_not_found() {
    let url = format!("passbolt://{}.passbolt.local/apps/rendered-webview/dist/nonexistent.js", RV_UUID);
    let response = request(&url, &AppMode::Import, None);
    assert_eq!(response.status(), 404);
}
