use super::ipc_state::IpcState;
use super::navigation_state::NavigationState;
use super::auth_state::AuthState;
use super::cookie_state::CookieState;
use super::credential_cache::CredentialCache;

/// Thread-safe application state shared across all event handlers.
#[derive(Default)]
pub struct AppState {
    pub ipc: IpcState,
    pub navigation: NavigationState,
    pub auth: AuthState,
    pub cookies: CookieState,
    pub credentials: CredentialCache,
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
