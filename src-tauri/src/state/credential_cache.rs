use std::sync::Mutex;

use log::trace;

use crate::models::account_metadata::AccountMetadata;
use crate::models::account_secret::AccountSecret;

/// Inner state protected by a single mutex so that `clear()` is atomic
/// and metadata/secret cannot be observed in an inconsistent state.
struct Inner {
    metadata: Option<AccountMetadata>,
    secret: Option<AccountSecret>,
}

/// In-memory cache for keyring credentials.
///
/// Avoids redundant OS keyring reads (and macOS Keychain prompts) within
/// a session. `AccountSecret` implements `ZeroizeOnDrop`, so assigning
/// `None` scrubs the private key from memory.
///
/// Uses a single mutex so `clear()` is atomic — no window where one
/// field is `None` while the other still holds a value.
pub struct CredentialCache {
    inner: Mutex<Inner>,
}

impl Default for CredentialCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                metadata: None,
                secret: None,
            }),
        }
    }

    pub fn set_metadata(&self, metadata: AccountMetadata) {
        self.inner.lock().unwrap().metadata = Some(metadata);
    }

    /// Returns a clone of the cached metadata.
    /// Metadata is not secret material, so cloning is acceptable.
    pub fn get_metadata(&self) -> Option<AccountMetadata> {
        let guard = self.inner.lock().unwrap();
        let result = guard.metadata.clone();
        if result.is_some() {
            trace!("[CredentialCache] metadata hit");
        } else {
            trace!("[CredentialCache] metadata miss");
        }
        result
    }

    pub fn set_secret(&self, secret: AccountSecret) {
        self.inner.lock().unwrap().secret = Some(secret);
    }

    /// Returns a clone of the cached secret.
    ///
    /// Each clone is an independent heap allocation of the private key.
    /// The caller is responsible for dropping it promptly; `ZeroizeOnDrop`
    /// on `AccountSecret` ensures the bytes are scrubbed when dropped.
    pub fn get_secret(&self) -> Option<AccountSecret> {
        let guard = self.inner.lock().unwrap();
        let result = guard.secret.clone();
        if result.is_some() {
            trace!("[CredentialCache] secret hit");
        } else {
            trace!("[CredentialCache] secret miss");
        }
        result
    }

    /// Access the secret without cloning. The callback receives a
    /// reference that cannot escape the lock scope.
    pub fn with_secret<R>(&self, callback: impl FnOnce(Option<&AccountSecret>) -> R) -> R {
        let guard = self.inner.lock().unwrap();
        callback(guard.secret.as_ref())
    }

    /// Atomically clear both metadata and secret.
    /// `ZeroizeOnDrop` on `AccountSecret` scrubs the private key on drop.
    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.metadata = None;
        guard.secret = None;
        trace!("[CredentialCache] cleared");
    }
}