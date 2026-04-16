use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::services::credential_service::{AccountMetaData, AccountSecret};

/// Cached credentials to avoid repeated keychain prompts.
pub struct CredentialCache {
    pub metadata: Option<AccountMetaData>,
    pub secret: Option<AccountSecret>,
}

pub struct AppState {
    request_ids: Mutex<HashSet<String>>,
    pending_requests: Mutex<HashMap<String, String>>,
    background_uuid: Mutex<String>,
    rendered_uuid: Mutex<String>,
    credentials: Mutex<CredentialCache>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            request_ids: Mutex::new(HashSet::new()),
            pending_requests: Mutex::new(HashMap::new()),
            background_uuid: Mutex::new(String::new()),
            rendered_uuid: Mutex::new(String::new()),
            credentials: Mutex::new(CredentialCache { metadata: None, secret: None }),
        }
    }

    pub fn proceed_request_id(&self, topic: &str) -> bool {
        let mut guard = self.request_ids.lock().unwrap_or_else(|e| e.into_inner());
        guard.remove(topic)
    }

    pub fn add_request_id(&self, request_id: &str) {
        let mut guard = self.request_ids.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(request_id.to_string());
    }

    pub fn add_pending_request(&self, request_id: &str, topic: &str) {
        let mut guard = self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(request_id.to_string(), topic.to_string());
    }

    pub fn has_pending_request(&self, topic: &str) -> bool {
        let guard = self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
        guard.contains_key(topic)
    }

    pub fn get_pending_request(&self, topic: &str) -> String {
        let guard = self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(topic).cloned().unwrap_or_default()
    }

    pub fn remove_pending_request(&self, topic: &str) {
        let mut guard = self.pending_requests.lock().unwrap_or_else(|e| e.into_inner());
        guard.remove(topic);
    }

    pub fn set_background_uuid(&self, uuid: &str) {
        let mut guard = self.background_uuid.lock().unwrap_or_else(|e| e.into_inner());
        *guard = uuid.to_string();
    }

    pub fn background_uuid(&self) -> String {
        let guard = self.background_uuid.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    pub fn set_rendered_uuid(&self, uuid: &str) {
        let mut guard = self.rendered_uuid.lock().unwrap_or_else(|e| e.into_inner());
        *guard = uuid.to_string();
    }

    pub fn rendered_uuid(&self) -> String {
        let guard = self.rendered_uuid.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    pub fn set_cached_credentials(&self, metadata: AccountMetaData, secret: AccountSecret) {
        let mut guard = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        guard.metadata = Some(metadata);
        guard.secret = Some(secret);
    }

    pub fn get_cached_metadata(&self) -> Option<AccountMetaData> {
        let guard = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        guard.metadata.clone()
    }

    pub fn get_cached_secret(&self) -> Option<AccountSecret> {
        let guard = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        guard.secret.clone()
    }

    pub fn update_cached_metadata(&self, f: impl FnOnce(&mut AccountMetaData)) {
        let mut guard = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut metadata) = guard.metadata {
            f(metadata);
        }
    }

    pub fn clear_cached_credentials(&self) {
        let mut guard = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        guard.metadata = None;
        guard.secret = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_proceed_request_id() {
        let state = AppState::new();
        state.add_request_id("req-1");
        assert!(state.proceed_request_id("req-1"));
        assert!(!state.proceed_request_id("req-1"));
    }

    #[test]
    fn test_unknown_request_id_returns_false() {
        let state = AppState::new();
        assert!(!state.proceed_request_id("nonexistent"));
    }

    #[test]
    fn test_pending_request_lifecycle() {
        let state = AppState::new();
        assert!(!state.has_pending_request("req-1"));

        state.add_pending_request("req-1", "topic-1");
        assert!(state.has_pending_request("req-1"));
        assert_eq!(state.get_pending_request("req-1"), "topic-1");

        state.remove_pending_request("req-1");
        assert!(!state.has_pending_request("req-1"));
        assert_eq!(state.get_pending_request("req-1"), "");
    }

    #[test]
    fn test_multiple_request_ids() {
        let state = AppState::new();
        state.add_request_id("req-1");
        state.add_request_id("req-2");
        assert!(state.proceed_request_id("req-1"));
        assert!(state.proceed_request_id("req-2"));
        assert!(!state.proceed_request_id("req-1"));
    }
}
