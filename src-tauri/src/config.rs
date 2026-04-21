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

/// Topics from Windows that need native handling.
pub const ERROR: &str = "passbolt.error";
pub const BACKGROUND_IMPORT: &str = "passbolt.background.import";
pub const BACKGROUND_AFTER_LOGOUT: &str = "passbolt.auth.after-logout";
pub const RENDERED_RELOAD: &str = "passbolt.tab.reload";
pub const BACKGROUND_CLIPBOARD_SET_TEXT: &str = "passbolt.background.clipboard-set-text";

/// IPC connect-src directives for Tauri event system (emit/listen).
pub const IPC_CONNECT_SRC: &str = "ipc://localhost/plugin:event|listen ipc://localhost/plugin:event|emit";
