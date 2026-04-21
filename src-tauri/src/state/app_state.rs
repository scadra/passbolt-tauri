use super::ipc_state::IpcState;
use super::navigation_state::NavigationState;
use super::auth_state::AuthState;
use super::cookie_state::CookieState;

/// Thread-safe application state shared across all event handlers.
///
/// Composed of focused sub-states:
/// - `ipc`: request/response correlation (request_ids, pending_requests, pending_messages)
/// - `navigation`: webview readiness, current mode, trusted domain
/// - `auth`: passphrase relay for the login→workspace transition
/// - `cookies`: in-memory cookie store (like WinUI 3's CookiesManager)
pub struct AppState {
    pub ipc: IpcState,
    pub navigation: NavigationState,
    pub auth: AuthState,
    pub cookies: CookieState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ipc: IpcState::new(),
            navigation: NavigationState::new(),
            auth: AuthState::new(),
            cookies: CookieState::new(),
        }
    }

    /// Reset ready flags and clear pending messages (called on mode transitions).
    pub fn reset_for_navigation(&self) {
        self.navigation.reset_ready_flags();
        self.ipc.clear_pending_messages();
    }
}
