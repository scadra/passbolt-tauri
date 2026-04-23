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
    let bundle_src = get_bundle_src(ctx.target, ctx.mode, ctx.uuid);
    let base = asset_base(
        match ctx.target {
            WebviewTarget::Rendered => "rendered-webview",
            WebviewTarget::Background => "background-webview",
        },
        ctx.uuid,
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
    {stylesheet_tag}
</head>
<body>
    <script src="{bundle_src}"></script>
</body>
</html>"#,
        uuid = ctx.uuid,
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

/// Base URL for a webview's assets: `passbolt://{uuid}.passbolt.local/apps/{webview}/`
fn asset_base(webview_dir: &str, uuid: &str) -> String {
    format!(
        "{}://{}{}/apps/{}/",
        config::PROTOCOL_SCHEME,
        uuid,
        config::DOMAIN_SUFFIX,
        webview_dir,
    )
}

/// `frame-src {isolation_schema}://localhost` — allows Tauri's isolation iframe.
fn frame_src_directive(isolation_schema: Option<&str>) -> String {
    match isolation_schema {
        Some(schema) => format!("frame-src {}://localhost", schema),
        None => String::new(),
    }
}

/// Rendered CSP — the visible UI webview.
///
/// Key security property: `connect-src` NEVER includes the API domain.
fn generate_rendered_csp(ctx: &IndexContext) -> String {
    let base = asset_base("rendered-webview", ctx.uuid);

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

    let origin = format!("{}://{}{}/", config::PROTOCOL_SCHEME, ctx.uuid, config::DOMAIN_SUFFIX);
    let connect_src = match ctx.mode {
        AppMode::Import => {
            format!("connect-src {}", config::IPC_CONNECT_SRC)
        }
        AppMode::Auth | AppMode::Workspace => {
            format!("connect-src {} {origin}", config::IPC_CONNECT_SRC)
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
    let base = asset_base("background-webview", ctx.uuid);

    let script_src = format!("script-src {base}");
    // The browser-extension injects inline styles — safe here (headless worker, no visible UI)
    let style_src = "style-src 'unsafe-inline'".to_string();

    let origin = format!("{}://{}{}/", config::PROTOCOL_SCHEME, ctx.uuid, config::DOMAIN_SUFFIX);
    let connect_src = format!("connect-src {} {origin}", config::IPC_CONNECT_SRC);

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

/// Returns the `passbolt://` URL for the JS bundle.
fn get_bundle_src(target: &WebviewTarget, mode: &AppMode, uuid: &str) -> String {
    let (webview_dir, name) = match target {
        WebviewTarget::Rendered => ("rendered-webview", "rendered"),
        WebviewTarget::Background => ("background-webview", "background"),
    };
    let mode_suffix = match mode {
        AppMode::Import => "import",
        AppMode::Auth => "auth",
        AppMode::Workspace => "workspace",
    };
    format!(
        "{}://{}{}/apps/{}/dist/{}-{}.js",
        config::PROTOCOL_SCHEME,
        uuid,
        config::DOMAIN_SUFFIX,
        webview_dir,
        name,
        mode_suffix,
    )
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

