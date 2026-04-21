use passbolt_app::models::ipc_message::IpcMessage;
use passbolt_app::models::messaging::AllowedTopics;
use passbolt_app::services::rendered_topic_service::is_valid_uuid;
use passbolt_app::state::AppState;

// ── Topic validation ────────────────────────────────────────────

/// Unknown topics are silently dropped — a compromised Rendered script
/// cannot invoke arbitrary handlers in Background.
#[test]
fn unknown_topic_is_rejected() {
    let topics = AllowedTopics::new();
    assert!(!topics.is_allowed("passbolt.evil.steal-data"));
}

/// Registered topics from the 33 topic modules pass validation.
#[test]
fn known_topic_is_accepted() {
    let topics = AllowedTopics::new();
    assert!(topics.is_allowed("passbolt.auth.login"));
}

/// Native topics (e.g. RENDERED_READY) also pass — they are added
/// to the whitelist from `config.rs` constants.
#[test]
fn native_topic_is_accepted() {
    let topics = AllowedTopics::new();
    assert!(topics.is_allowed("passbolt.rendered.is-ready"));
}

// ── RequestId validation ────────────────────────────────────────

/// Valid UUIDs pass the regex check and get registered in the whitelist.
#[test]
fn valid_request_id_passes_validation() {
    assert!(is_valid_uuid("8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a55"));
}

/// Invalid requestIds are rejected before they can be registered.
/// This prevents injection of arbitrary strings into the request whitelist.
#[test]
fn invalid_request_id_is_rejected() {
    assert!(!is_valid_uuid("not-a-uuid"));
    assert!(!is_valid_uuid(""));
    assert!(!is_valid_uuid("DROP TABLE users;"));
}

// ── RequestId registration ──────────────────────────────────────

/// When a valid IPC message arrives with a requestId, the service
/// registers it in the one-shot whitelist so the matching response
/// from Background will pass validation.
#[test]
fn valid_request_id_is_registered_in_whitelist() {
    let state = AppState::new();

    let ipc = IpcMessage {
        topic: "passbolt.auth.login".to_string(),
        status: None,
        message: None,
        request_id: Some("8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a55".to_string()),
        with_multiple_param: false,
    };

    if let Some(ref id) = ipc.request_id {
        if is_valid_uuid(id) {
            state.ipc.add_request_id(id);
        }
    }

    assert!(state.ipc.proceed_request_id("8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a55"));
}

/// An invalid UUID in the requestId field must NOT be registered.
/// The message is dropped entirely.
#[test]
fn invalid_request_id_is_not_registered() {
    let state = AppState::new();

    let ipc = IpcMessage {
        topic: "passbolt.auth.login".to_string(),
        status: None,
        message: None,
        request_id: Some("invalid-uuid".to_string()),
        with_multiple_param: false,
    };

    if let Some(ref id) = ipc.request_id {
        if is_valid_uuid(id) {
            state.ipc.add_request_id(id);
        }
    }

    assert!(!state.ipc.proceed_request_id("invalid-uuid"));
}

// ── RENDERED_READY ──────────────────────────────────────────────

/// When RENDERED_READY fires, the ready flag is set and all pending
/// messages (buffered while Rendered was booting) are drained.
/// After drain, the buffer is empty.
#[test]
fn rendered_ready_sets_flag_and_drains_pending_messages() {
    let state = AppState::new();

    state.ipc.push_pending_message(IpcMessage::new("passbolt.background.localstorage-update", None));
    state.ipc.push_pending_message(IpcMessage::new("passbolt.background.localstorage-update", None));

    state.navigation.set_rendered_ready(true);
    let messages = state.ipc.drain_pending_messages();

    assert!(state.navigation.is_rendered_ready());
    assert_eq!(messages.len(), 2);
    assert_eq!(state.ipc.drain_pending_messages().len(), 0);
}

// ── RBAC interception ───────────────────────────────────────────

/// When `passbolt.rbacs.find-me` arrives, the requestId is registered
/// in `pending_requests` so BackgroundTopicService can post-process
/// the response (inject desktop-specific deny entries).
#[test]
fn rbac_request_registers_pending_request_for_interception() {
    let state = AppState::new();

    let ipc = IpcMessage {
        topic: "passbolt.rbacs.find-me".to_string(),
        status: None,
        message: None,
        request_id: Some("rbac-req-123".to_string()),
        with_multiple_param: false,
    };

    if let Some(ref id) = ipc.request_id {
        state.ipc.add_pending_request(id, &ipc.topic);
    }

    assert!(state.ipc.has_pending_request("rbac-req-123"));
    assert_eq!(
        state.ipc.get_pending_request("rbac-req-123"),
        Some("passbolt.rbacs.find-me".to_string())
    );
}

// ── End-to-end lifecycle ────────────────────────────────────────

/// Full request/response correlation:
/// 1. Rendered sends a request → requestId registered
/// 2. Background responds (topic = requestId) → consumed
/// 3. Replay blocked (one-shot)
#[test]
fn request_response_lifecycle_is_one_shot() {
    let state = AppState::new();

    let request_id = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    state.ipc.add_request_id(request_id);

    assert!(state.ipc.proceed_request_id(request_id));
    assert!(!state.ipc.proceed_request_id(request_id));
}

/// Full RBAC interception lifecycle:
/// 1. Rendered sends `passbolt.rbacs.find-me` → requestId + pending_request registered
/// 2. Background responds → requestId consumed, original topic recovered
/// 3. Post-processing applied, pending_request cleaned up
#[test]
fn rbac_intercept_lifecycle() {
    let state = AppState::new();

    let request_id = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

    state.ipc.add_request_id(request_id);
    state.ipc.add_pending_request(request_id, "passbolt.rbacs.find-me");

    assert!(state.ipc.proceed_request_id(request_id));

    let original = state.ipc.get_pending_request(request_id);
    assert_eq!(original, Some("passbolt.rbacs.find-me".to_string()));

    state.ipc.remove_pending_request(request_id);
    assert!(!state.ipc.has_pending_request(request_id));
}
