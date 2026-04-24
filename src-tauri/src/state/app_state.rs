use super::ipc_state::IpcState;
use super::navigation_state::NavigationState;
use super::auth_state::AuthState;
use super::cookie_state::CookieState;
use super::credential_cache::CredentialCache;

/// Thread-safe application state shared across all event handlers.
pub struct AppState {
    pub ipc: IpcState,
    pub navigation: NavigationState,
    pub auth: AuthState,
    pub cookies: CookieState,
    pub credentials: CredentialCache,
    /// Shared HTTP client — reuses connections (TLS session, TCP socket).
    /// Created once at startup, used by all fetch requests.
    pub http_client: reqwest::Client,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            ipc: IpcState::default(),
            navigation: NavigationState::default(),
            auth: AuthState::default(),
            cookies: CookieState::default(),
            credentials: CredentialCache::default(),
            http_client: reqwest::Client::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset ready flags and clear pending messages (called on mode transitions).
    pub fn reset_for_navigation(&self) {
        self.navigation.reset_ready_flags();
        self.ipc.clear_pending_messages();
    }
}
