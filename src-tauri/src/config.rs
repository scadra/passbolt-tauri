pub const BACKGROUND_WEBVIEW_LABEL: &str = "background-webview";
pub const RENDERED_WEBVIEW_LABEL: &str = "rendered-webview";
pub const DOMAIN_SUFFIX: &str = ".passbolt.local";
pub const PROTOCOL_SCHEME: &str = "passbolt";
pub const WINDOW_DEFAULT_WIDTH: f64 = 1200.0;
pub const WINDOW_DEFAULT_HEIGHT: f64 = 800.0;
pub const WINDOW_MIN_WIDTH: f64 = 800.0;
pub const WINDOW_MIN_HEIGHT: f64 = 600.0;
pub const HELP_URL_PREFIX: &str = "https://help.passbolt.com/";
// JS → Rust (listeners)
pub const RENDERED_IPC_EVENT: &str = "rendered-ipc-message";
pub const BACKGROUND_IPC_EVENT: &str = "background-ipc-message";
// Rust → JS (forwarding)
pub const RENDERED_IPC_FORWARD: &str = "rendered-ipc-forward";
pub const BACKGROUND_IPC_FORWARD: &str = "background-ipc-forward";
