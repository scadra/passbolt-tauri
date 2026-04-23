/// Custom protocol scheme for serving webview content.
pub const PROTOCOL_SCHEME: &str = "passbolt";

/// Domain suffix for virtual host validation.
pub const DOMAIN_SUFFIX: &str = ".passbolt.local";

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
// On Windows, WebView2 maps custom protocol schemes to HTTP:
//   passbolt://host/path  →  http://passbolt.host/path
//   ipc://localhost/cmd   →  http://ipc.localhost/cmd
//   isolation-x://localhost → http://isolation-x.localhost
//
// These helpers generate URLs as the **browser** sees them, used for
// CSP directives and HTML content (script src, etc.).
// The Rust protocol handler is NOT affected (Tauri normalises back).

/// Build a URL in the format the browser will see on the current platform.
///
/// macOS (WKWebView):  `{scheme}://{host}{path}`
/// Windows (WebView2): `http://{scheme}.{host}{path}`
pub fn browser_url(scheme: &str, host: &str, path: &str) -> String {
    #[cfg(target_os = "windows")]
    { format!("http://{}.{}{}", scheme, host, path) }

    #[cfg(not(target_os = "windows"))]
    { format!("{}://{}{}", scheme, host, path) }
}

/// Browser-visible origin for a webview: `passbolt://uuid.passbolt.local/` (macOS)
/// or `http://passbolt.uuid.passbolt.local/` (Windows).
pub fn browser_origin(uuid: &str) -> String {
    browser_url(PROTOCOL_SCHEME, &format!("{}{}", uuid, DOMAIN_SUFFIX), "/")
}

/// IPC connect-src for CSP — platform-aware.
pub fn ipc_connect_src() -> &'static str {
    #[cfg(target_os = "windows")]
    { "http://ipc.localhost/plugin:event|listen http://ipc.localhost/plugin:event|emit" }

    #[cfg(not(target_os = "windows"))]
    { "ipc://localhost/plugin:event|listen ipc://localhost/plugin:event|emit" }
}

/// `frame-src` value for the Tauri isolation iframe — platform-aware.
pub fn isolation_frame_src(schema: &str) -> String {
    #[cfg(target_os = "windows")]
    { format!("http://{}.localhost", schema) }

    #[cfg(not(target_os = "windows"))]
    { format!("{}://localhost", schema) }
}
