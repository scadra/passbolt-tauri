/// Custom protocol scheme for serving webview content.
pub const PROTOCOL_SCHEME: &str = "passbolt";

/// Domain suffix for virtual host validation.
///
/// Uses `.passbolt.localhost` (not `.passbolt.local`) because on Windows, WebView2
/// maps custom protocols to `http://{scheme}.{host}/`. Only `*.localhost` domains
/// are treated as secure contexts in Chromium, which is required for `crypto.subtle`
/// (used by Tauri's isolation bridge). On macOS this makes no difference — WKWebView
/// treats custom scheme URLs as secure regardless of the domain suffix.
pub const DOMAIN_SUFFIX: &str = ".passbolt.localhost";

/// Webview labels — must match capabilities/default.json exactly.
pub const RENDERED_WEBVIEW_LABEL: &str = "rendered-webview";
pub const BACKGROUND_WEBVIEW_LABEL: &str = "background-webview";

/// Event names for IPC between JS and Rust.
pub const RENDERED_IPC_MESSAGE: &str = "rendered-ipc-message";
pub const RENDERED_IPC_FORWARD: &str = "rendered-ipc-forward";
pub const BACKGROUND_IPC_MESSAGE: &str = "background-ipc-message";
pub const BACKGROUND_IPC_FORWARD: &str = "background-ipc-forward";

/// Native-handled topics (not forwarded, handled by Rust directly).
pub const BACKGROUND_READY: &str = "passbolt.background.is-ready";
pub const RENDERED_READY: &str = "passbolt.rendered.is-ready";
pub const BACKGROUND_LISTENER_READY: &str = "passbolt.background.listener-ready";
pub const BACKGROUND_STORE_PASSPHRASE: &str = "passbolt.background.store-passphrase";
pub const BACKGROUND_AUTHENTICATION_ERROR: &str = "passbolt.background.authentication-error";
pub const BACKGROUND_SET_THEME: &str = "passbolt.background.set-theme";
pub const BACKGROUND_SET_LOCALE: &str = "passbolt.background.set-locale";
pub const BACKGROUND_SET_SECURITY_TOKEN: &str = "passbolt.background.set-security-token";
pub const BACKGROUND_ROTATE_KEY: &str = "passbolt.background.rotate-private-key";
pub const BACKGROUND_DOWNLOAD_FILE: &str = "passbolt.background.download-file";
pub const BACKGROUND_GET_COOKIE: &str = "passbolt.background.get-cookie";
pub const BACKGROUND_CLIPBOARD: &str = "passbolt.clipboard.copy";
pub const ACCOUNT_SET_CURRENT: &str = "passbolt.account.set-current";
pub const SAVE_ACCOUNT: &str = "passbolt.background.save-account";
pub const BACKGROUND_HTTP_FETCH: &str = "passbolt.http.fetch";
pub const RENDERED_ASSET_FETCH: &str = "passbolt.rendered.asset-fetch";

/// Topics from Windows that need native handling.
pub const ERROR: &str = "passbolt.error";
pub const BACKGROUND_IMPORT: &str = "passbolt.background.import";
pub const BACKGROUND_AFTER_LOGOUT: &str = "passbolt.auth.after-logout";
pub const RENDERED_RELOAD: &str = "passbolt.tab.reload";
pub const BACKGROUND_CLIPBOARD_SET_TEXT: &str = "passbolt.background.clipboard-set-text";

// ── Browser-facing URL helpers ──────────────────────────────────
//
// On Windows with useHttpsScheme, WebView2 maps custom protocols to HTTPS:
//   passbolt://host/path  →  https://passbolt.host/path
//   ipc://localhost/cmd   →  https://ipc.localhost/cmd
//   isolation-x://localhost → https://isolation-x.localhost
//
// On Windows auth/workspace, the domain matches the API server for cookie sharing:
//   passbolt://uuid.cloud.passbolt.com/path → https://passbolt.uuid.cloud.passbolt.com/path
//   (subdomain of cloud.passbolt.com → cookies shared automatically)
//
// On macOS, custom schemes work natively: passbolt://uuid.passbolt.localhost/path
// The Rust protocol handler is NOT affected (Tauri normalises back).

/// The scheme prefix used by WebView2 for custom protocol URLs.
#[cfg(target_os = "windows")]
const BROWSER_SCHEME: &str = "https";
#[cfg(not(target_os = "windows"))]
const BROWSER_SCHEME: &str = "passbolt";

/// Extract the domain suffix from a trusted domain URL for cookie sharing.
///
/// `"https://cloud.passbolt.com"` → `".cloud.passbolt.com"`
///
/// On macOS or when no trusted domain is available, returns `DOMAIN_SUFFIX`.
pub fn effective_domain_suffix(trusted_domain: Option<&str>) -> String {
    if let Some(domain) = trusted_domain {
        if let Ok(parsed) = url::Url::parse(domain) {
            if let Some(host) = parsed.host_str() {
                return format!(".{}", host);
            }
        }
    }
    DOMAIN_SUFFIX.to_string()
}

/// Build a URL in the format the browser will see on the current platform.
///
/// macOS:   `passbolt://{host}{path}`
/// Windows: `https://passbolt.{host}{path}`
pub fn browser_url(host: &str, path: &str) -> String {
    #[cfg(target_os = "windows")]
    { format!("{}://passbolt.{}{}", BROWSER_SCHEME, host, path) }

    #[cfg(not(target_os = "windows"))]
    { format!("{}://{}{}", BROWSER_SCHEME, host, path) }
}

/// Browser-visible origin for a webview, using the effective domain suffix.
///
/// macOS:   `passbolt://uuid.passbolt.localhost/`
/// Windows: `https://passbolt.uuid.cloud.passbolt.com/` (with trusted domain)
///          `https://passbolt.uuid.passbolt.localhost/`  (import, no trusted domain)
pub fn browser_origin(uuid: &str, trusted_domain: Option<&str>) -> String {
    let suffix = effective_domain_suffix(trusted_domain);
    browser_url(&format!("{}{}", uuid, suffix), "/")
}

/// Build the full webview host (without scheme prefix).
///
/// macOS:   `uuid.passbolt.localhost`
/// Windows: `uuid.cloud.passbolt.com` (with trusted domain)
pub fn webview_host(uuid: &str, trusted_domain: Option<&str>) -> String {
    let suffix = effective_domain_suffix(trusted_domain);
    format!("{}{}", uuid, suffix)
}

/// IPC connect-src for CSP — platform-aware.
pub fn ipc_connect_src() -> &'static str {
    #[cfg(target_os = "windows")]
    { "https://ipc.localhost/plugin:event|listen https://ipc.localhost/plugin:event|emit https://ipc.localhost/plugin:event|unlisten" }

    #[cfg(not(target_os = "windows"))]
    { "ipc://localhost/plugin:event|listen ipc://localhost/plugin:event|emit ipc://localhost/plugin:event|unlisten" }
}

/// `frame-src` value for the Tauri isolation iframe — platform-aware.
pub fn isolation_frame_src(schema: &str) -> String {
    #[cfg(target_os = "windows")]
    { format!("https://{}.localhost", schema) }

    #[cfg(not(target_os = "windows"))]
    { format!("{}://localhost", schema) }
}

/// Convert a `passbolt://` internal URL to the browser-navigable format.
///
/// On macOS, returns the URL unchanged (WKWebView handles custom schemes natively).
/// On Windows, rewrites to `https://passbolt.{host}/{path}` for WebView2 navigation.
pub fn to_navigation_url(url: &str) -> String {
    #[cfg(target_os = "windows")]
    { url.replace("passbolt://", "https://passbolt.") }

    #[cfg(not(target_os = "windows"))]
    { url.to_string() }
}
