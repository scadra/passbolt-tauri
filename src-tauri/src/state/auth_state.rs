use std::sync::Mutex;
use zeroize::Zeroizing;

/// Passphrase relay for the 2-hop auth→workspace transition.
///
/// The passphrase is cached here between two Background contexts:
/// 1. Auth-Background validates it and sends `AFTER_LOGIN` → Rust caches it here
/// 2. Workspace-Background loads and signals `BACKGROUND_READY` → Rust delivers it
///
/// The passphrase is never persisted to disk. It only lives in memory
/// for the duration of the mode transition. `Zeroizing` ensures the bytes
/// are wiped on drop, preventing forensic recovery from freed memory.
pub struct AuthState {
    passphrase: Mutex<Option<Zeroizing<String>>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            passphrase: Mutex::new(None),
        }
    }
}

impl AuthState {

    /// Cache the passphrase for the 2-hop relay.
    pub fn set_passphrase(&self, passphrase: String) {
        *self.passphrase.lock().unwrap() = Some(Zeroizing::new(passphrase));
    }

    /// Take the passphrase (one-shot). Returns None if already consumed.
    /// The `Zeroizing` wrapper is dropped here, wiping the cached bytes.
    pub fn take_passphrase(&self) -> Option<String> {
        self.passphrase.lock().unwrap().take().map(|zeroizing| zeroizing.as_str().to_owned())
    }
}
