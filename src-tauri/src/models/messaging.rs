pub use crate::models::ipc_model::IPC;

pub struct AllowedTopics;

impl AllowedTopics {
    pub const RENDERED_READY: &'static str = "passbolt.rendered.is-ready";
    pub const RENDERED_RELOAD: &'static str = "passbolt.tab.reload";
    pub const BACKGROUND_READY: &'static str = "passbolt.background.is-ready";
    pub const BACKGROUND_AFTER_LOGOUT: &'static str = "passbolt.auth.after-logout";
    pub const BACKGROUND_GET_COOKIE: &'static str = "passbolt.background.get-cookie";
    pub const BACKGROUND_SET_THEME: &'static str = "passbolt.background.set-theme";
    pub const BACKGROUND_SET_LOCALE: &'static str = "passbolt.background.set-locale";
    pub const BACKGROUND_SET_SECURITY_TOKEN: &'static str = "passbolt.background.set-security-token";
    pub const BACKGROUND_ROTATE_KEY: &'static str = "passbolt.background.rotate-private-key";
    pub const BACKGROUND_AUTHENTICATION_ERROR: &'static str = "passbolt.background.authentication-error";
    pub const BACKGROUND_DOWNLOAD_FILE: &'static str = "passbolt.background.download-file";
    pub const BACKGROUND_CLIPBOARD_SET_TEXT: &'static str = "passbolt.background.clipboard-set-text";
    pub const BACKGROUND_STORE_PASSPHRASE: &'static str = "passbolt.background.store-passphrase";
    pub const ERROR: &'static str = "passbolt.error";
    pub const BACKGROUND_IMPORT: &'static str = "passbolt.background.import";
    pub const BACKGROUND_LISTENER_READY: &'static str = "passbolt.background.listener-ready";

    /// Returns all AllowedTopics constants for use in topic whitelists.
    pub fn all() -> &'static [&'static str] {
        &[
            Self::RENDERED_READY,
            Self::RENDERED_RELOAD,
            Self::BACKGROUND_READY,
            Self::BACKGROUND_AFTER_LOGOUT,
            Self::BACKGROUND_GET_COOKIE,
            Self::BACKGROUND_SET_THEME,
            Self::BACKGROUND_SET_LOCALE,
            Self::BACKGROUND_SET_SECURITY_TOKEN,
            Self::BACKGROUND_ROTATE_KEY,
            Self::BACKGROUND_AUTHENTICATION_ERROR,
            Self::BACKGROUND_DOWNLOAD_FILE,
            Self::BACKGROUND_CLIPBOARD_SET_TEXT,
            Self::BACKGROUND_STORE_PASSPHRASE,
            Self::ERROR,
            Self::BACKGROUND_IMPORT,
            Self::BACKGROUND_LISTENER_READY,
        ]
    }
}
