use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::models::ipc_message::IpcMessage;

/// Maximum age of a request ID before it is considered expired and eligible for cleanup.
const REQUEST_ID_TTL_SECONDS: u64 = 300;

/// IPC request/response correlation state.
///
/// Manages the three collections that coordinate the async IPC protocol:
/// - `request_ids`: one-shot whitelist of requestIds awaiting a response (with creation timestamp for TTL)
/// - `pending_requests`: requestId → original topic mapping for response interception
/// - `pending_messages`: buffer for Background→Rendered messages while Rendered boots
pub struct IpcState {
    request_ids: Mutex<HashMap<String, Instant>>,
    pending_requests: Mutex<HashMap<String, String>>,
    pending_messages: Mutex<Vec<IpcMessage>>,
}

impl Default for IpcState {
    fn default() -> Self {
        Self {
            request_ids: Mutex::new(HashMap::new()),
            pending_requests: Mutex::new(HashMap::new()),
            pending_messages: Mutex::new(Vec::new()),
        }
    }
}

impl IpcState {

    // --- Request ID whitelist ---

    /// Register a requestId so the matching response passes validation.
    /// Also triggers cleanup of expired request IDs (piggyback approach, no separate timer needed).
    pub fn add_request_id(&self, id: &str) {
        self.cleanup_expired_request_ids();
        self.request_ids.lock().unwrap().insert(id.to_string(), Instant::now());
    }

    /// Consume a request ID (one-shot). Returns true if it was present.
    pub fn proceed_request_id(&self, id: &str) -> bool {
        self.request_ids.lock().unwrap().remove(id).is_some()
    }

    /// Remove request IDs older than the TTL.
    /// Called as piggyback on `add_request_id` to prevent leaked IDs from accumulating.
    fn cleanup_expired_request_ids(&self) {
        let ttl = std::time::Duration::from_secs(REQUEST_ID_TTL_SECONDS);
        let mut request_ids = self.request_ids.lock().unwrap();
        let before_count = request_ids.len();
        request_ids.retain(|_id, created_at| created_at.elapsed() < ttl);
        let removed_count = before_count - request_ids.len();
        if removed_count > 0 {
            log::debug!("[IPC] Cleaned up {} expired request IDs", removed_count);
        }
    }

    // --- Pending requests (response interception) ---

    /// Register a request for post-processing when its response arrives.
    pub fn add_pending_request(&self, request_id: &str, topic: &str) {
        self.pending_requests
            .lock()
            .unwrap()
            .insert(request_id.to_string(), topic.to_string());
    }

    pub fn has_pending_request(&self, request_id: &str) -> bool {
        self.pending_requests.lock().unwrap().contains_key(request_id)
    }

    /// Retrieve the original topic for a pending request.
    pub fn get_pending_request(&self, request_id: &str) -> Option<String> {
        self.pending_requests.lock().unwrap().get(request_id).cloned()
    }

    pub fn remove_pending_request(&self, request_id: &str) {
        self.pending_requests.lock().unwrap().remove(request_id);
    }

    // --- Pending messages (buffer while Rendered boots) ---

    /// Buffer a message for later delivery to Rendered.
    pub fn push_pending_message(&self, msg: IpcMessage) {
        self.pending_messages.lock().unwrap().push(msg);
    }

    /// Drain all buffered messages (called when Rendered signals ready).
    pub fn drain_pending_messages(&self) -> Vec<IpcMessage> {
        self.pending_messages.lock().unwrap().drain(..).collect()
    }

    /// Clear all buffered messages (called on mode transitions).
    pub fn clear_pending_messages(&self) {
        self.pending_messages.lock().unwrap().clear();
    }

    /// Find and remove the first pending request whose topic matches.
    ///
    /// Returns the request ID (the map key), not the topic (the map value),
    /// because the caller already knows the topic — it needs the request ID
    /// to resolve the original Promise on the JS side.
    ///
    /// If multiple entries share the same topic, only the first arbitrary
    /// match is removed (HashMap iteration order is unspecified).
    pub fn take_pending_request_by_topic(&self, topic: &str) -> Option<String> {
        let mut map = self.pending_requests.lock().unwrap();
        let key = map.iter()
            .find(|(_, value)| value.as_str() == topic)
            .map(|(request_id, _)| request_id.clone())?;
        map.remove(&key);
        Some(key)
    }
}
