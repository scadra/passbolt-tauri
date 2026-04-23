// ── Topic routing (message dispatch) ──────────────────────────
pub mod topics;
pub mod rendered_topic_service;
pub mod background_topic_service;

// ── Domain services (process IPC payloads) ────────────────────
pub mod auth_service;
pub mod account_service;
pub mod settings_service;
pub mod mfa_service;
pub mod rbac_service;
pub mod http_fetch_service;
pub mod cookie_service;
pub mod clipboard_service;
pub mod download_service;
pub mod asset_service;

// ── Infrastructure services (app lifecycle) ───────────────────
pub mod webview_service;
pub mod ipc_listener_service;
pub mod protocol_service;
pub mod local_folder_service;
pub mod navigation_service;
pub mod keyring_service;

// ── Platform utilities ────────────────────────────────────────
pub mod webview_security;
pub mod url_validation;
