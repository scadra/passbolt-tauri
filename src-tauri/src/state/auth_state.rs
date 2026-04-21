use std::sync::Mutex;

/// Passphrase relay for the 2-hop auth→workspace transition.
///
/// The passphrase is cached here between two Background contexts:
/// 1. Auth-Background validates it and sends `AFTER_LOGIN` → Rust caches it here
/// 2. Workspace-Background loads and signals `BACKGROUND_READY` → Rust delivers it
///
/// The passphrase is never persisted to disk. It only lives in memory
/// for the duration of the mode transition.
pub struct AuthState {
    passphrase: Mutex<Option<String>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            passphrase: Mutex::new(None),
        }
    }

    /// Cache the passphrase for the 2-hop relay.
    pub fn set_passphrase(&self, passphrase: String) {
        *self.passphrase.lock().unwrap() = Some(passphrase);
    }

    /// Take the passphrase (one-shot). Returns None if already consumed.
    pub fn take_passphrase(&self) -> Option<String> {
        self.passphrase.lock().unwrap().take()
    }
}
