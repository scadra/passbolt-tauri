//! Generates dynamic `index-*.html` with per-mode, per-webview CSP.
//!
//! Each webview gets a unique UUID origin: `passbolt://{uuid}.passbolt.local/...`
//! This ensures Rendered cannot load resources from Background's origin and vice versa.

use crate::config;
use crate::state::AppMode;

/// Which webview the HTML is generated for.
#[derive(Debug, Clone, PartialEq)]
pub enum WebviewTarget {
    Rendered,
    Background,
}

/// All context needed to generate a CSP-hardened index HTML.
pub struct IndexContext<'a> {
    pub target: &'a WebviewTarget,
    pub mode: &'a AppMode,
    pub uuid: &'a str,
    pub trusted_domain: Option<&'a str>,
    pub isolation_schema: Option<&'a str>,
}

/// Generate the dynamic `index.html` for a given webview and mode.
pub fn generate_index_html(ctx: &IndexContext) -> String {
    let csp = generate_csp(ctx);
    let bundle_src = get_bundle_src(ctx.target, ctx.mode, ctx.uuid, ctx.trusted_domain);
    let base = asset_base(
        match ctx.target {
            WebviewTarget::Rendered => "rendered-webview",
            WebviewTarget::Background => "background-webview",
        },
        ctx.uuid,
        ctx.trusted_domain,
    );

    // Rendered gets the stylesheet manager that dynamically loads the CSS theme
    let stylesheet_tag = match ctx.target {
        WebviewTarget::Rendered => {
            let css_file = get_stylesheet_for_mode(ctx.mode);
            format!(
                r#"<script id="stylesheet-manager" data-file="{css_file}" src="{base}src/js/Stylesheet.js"></script>"#,
            )
        }
        WebviewTarget::Background => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta http-equiv="Content-Security-Policy" content="{csp}" />
    <meta name="instance-uuid" content="{uuid}" />
    <meta name="fetch-proxy" content="{fetch_proxy}" />
    <style>#__tauri_isolation__ {{ display: none !important; }}</style>
    {stylesheet_tag}
</head>
<body>
    <script src="{bundle_src}"></script>
</body>
</html>"#,
        uuid = ctx.uuid,
        fetch_proxy = if cfg!(target_os = "windows") { "disabled" } else { "enabled" },
    )
}

/// Returns the CSS filename for a given mode.
fn get_stylesheet_for_mode(mode: &AppMode) -> &'static str {
    match mode {
        AppMode::Import => "ext_authentication.min.css",
        AppMode::Auth => "ext_authentication.min.css",
        AppMode::Workspace => "ext_app.min.css",
    }
}

/// Generate the CSP string for a given webview and mode.
pub fn generate_csp(ctx: &IndexContext) -> String {
    match ctx.target {
        WebviewTarget::Rendered => generate_rendered_csp(ctx),
        WebviewTarget::Background => generate_background_csp(ctx),
    }
}

/// Base URL for a webview's assets, in the format the browser sees.
///
/// macOS:   `passbolt://{uuid}.passbolt.localhost/apps/{webview}/`
/// Windows: `https://passbolt.{uuid}.cloud.passbolt.com/apps/{webview}/` (with trusted domain)
fn asset_base(webview_dir: &str, uuid: &str, trusted_domain: Option<&str>) -> String {
    let host = config::webview_host(uuid, trusted_domain);
    config::browser_url(&host, &format!("/apps/{}/", webview_dir))
}

/// `frame-src` directive for Tauri's isolation iframe — platform-aware.
///
/// macOS:   `frame-src isolation-xxx://localhost`
/// Windows: `frame-src http://isolation-xxx.localhost`
fn frame_src_directive(isolation_schema: Option<&str>) -> String {
    match isolation_schema {
        Some(schema) => format!("frame-src {}", config::isolation_frame_src(schema)),
        None => String::new(),
    }
}

/// Rendered CSP — the visible UI webview.
///
/// Key security property: `connect-src` NEVER includes the API domain.
fn generate_rendered_csp(ctx: &IndexContext) -> String {
    let base = asset_base("rendered-webview", ctx.uuid, ctx.trusted_domain);

    let script_src = format!("script-src {base}");
    let style_src = format!("style-src 'self' 'unsafe-inline' {base}dist/themes/");
    let font_src = format!("font-src {base}fonts/");

    let img_src = match ctx.mode {
        AppMode::Import => format!("img-src {base}img/"),
        AppMode::Auth => {
            let domain = format_trusted_domain(ctx.trusted_domain);
            format!("img-src {base}img/ {domain}")
        }
        AppMode::Workspace => {
            let domain = format_trusted_domain(ctx.trusted_domain);
            format!("img-src {base}img/ {domain} data: blob:")
        }
    };

    let origin = config::browser_origin(ctx.uuid, ctx.trusted_domain);
    let connect_src = match ctx.mode {
        AppMode::Import => {
            format!("connect-src {}", config::ipc_connect_src())
        }
        AppMode::Auth | AppMode::Workspace => {
            format!("connect-src {} {origin}", config::ipc_connect_src())
        }
    };

    let frame_src = frame_src_directive(ctx.isolation_schema);

    let mut parts = vec![
        "default-src 'none'".to_string(),
        "base-uri 'none'".to_string(),
        "object-src 'none'".to_string(),
        "form-action 'none'".to_string(),
    ];
    if !frame_src.is_empty() {
        parts.push(frame_src);
    }
    parts.extend([script_src, style_src, font_src, img_src, connect_src]);
    parts.join("; ")
}

/// Background CSP — the hidden crypto worker webview.
///
/// Key security property: `connect-src` only allows IPC and local assets.
/// All HTTP requests go through the Rust proxy via IPC events, so no HTTPS
/// domain needs to be whitelisted here.
fn generate_background_csp(ctx: &IndexContext) -> String {
    let base = asset_base("background-webview", ctx.uuid, ctx.trusted_domain);

    let script_src = format!("script-src {base}");
    // The browser-extension injects inline styles — safe here (headless worker, no visible UI)
    let style_src = "style-src 'unsafe-inline'".to_string();

    let origin = config::browser_origin(ctx.uuid, ctx.trusted_domain);

    // On Windows, the Background webview makes API calls directly through the browser
    // (fetch polyfill disabled), so the trusted domain must be in connect-src.
    #[cfg(target_os = "windows")]
    let connect_src = {
        let api_domain = format_trusted_domain(ctx.trusted_domain);
        format!("connect-src {} {origin} {api_domain}", config::ipc_connect_src())
    };
    #[cfg(not(target_os = "windows"))]
    let connect_src = format!("connect-src {} {origin}", config::ipc_connect_src());

    let frame_src = frame_src_directive(ctx.isolation_schema);

    let mut parts = vec![
        "default-src 'none'".to_string(),
        "base-uri 'none'".to_string(),
        "object-src 'none'".to_string(),
        "form-action 'none'".to_string(),
    ];
    if !frame_src.is_empty() {
        parts.push(frame_src);
    }
    parts.extend([script_src, style_src, connect_src]);
    parts.join("; ")
}

/// Returns the browser-visible URL for the JS bundle.
fn get_bundle_src(target: &WebviewTarget, mode: &AppMode, uuid: &str, trusted_domain: Option<&str>) -> String {
    let (webview_dir, name) = match target {
        WebviewTarget::Rendered => ("rendered-webview", "rendered"),
        WebviewTarget::Background => ("background-webview", "background"),
    };
    let mode_suffix = match mode {
        AppMode::Import => "import",
        AppMode::Auth => "auth",
        AppMode::Workspace => "workspace",
    };
    let host = config::webview_host(uuid, trusted_domain);
    config::browser_url(&host, &format!("/apps/{}/dist/{}-{}.js", webview_dir, name, mode_suffix))
}

/// Format trusted domain with trailing slash, or empty string if None.
fn format_trusted_domain(trusted_domain: Option<&str>) -> String {
    match trusted_domain {
        Some(domain) if !domain.is_empty() => {
            if domain.ends_with('/') {
                domain.to_string()
            } else {
                format!("{}/", domain)
            }
        }
        _ => String::new(),
    }
}

