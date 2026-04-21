use passbolt_app::state::{AppMode, AppState};

// ── IPC state ───────────────────────────────────────────────────

/// A requestId is consumed on first `proceed_request_id` (one-shot).
/// A second call with the same ID returns false — this prevents
/// response replay attacks.
#[test]
fn request_id_is_consumed_on_first_use() {
    let state = AppState::new();

    state.ipc.add_request_id("valid-request");
    assert!(state.ipc.proceed_request_id("valid-request"));
    assert!(!state.ipc.proceed_request_id("valid-request"));
}

/// Pending requests map a requestId to its original topic.
/// After removal, the entry must no longer be found.
#[test]
fn pending_request_is_removed_after_cleanup() {
    let state = AppState::new();

    state.ipc.add_pending_request("req-1", "topic-1");
    assert!(state.ipc.has_pending_request("req-1"));

    state.ipc.remove_pending_request("req-1");
    assert!(!state.ipc.has_pending_request("req-1"));
}

/// `get_pending_request` returns the original topic that was registered,
/// so BackgroundTopicService can decide how to post-process the response.
#[test]
fn pending_request_returns_original_topic() {
    let state = AppState::new();

    state.ipc.add_pending_request("req-1", "passbolt.rbacs.find-me");
    assert_eq!(
        state.ipc.get_pending_request("req-1"),
        Some("passbolt.rbacs.find-me".to_string())
    );
}

/// Pending messages are buffered while Rendered boots. `drain` returns
/// all of them in order and empties the buffer.
#[test]
fn pending_messages_drain_in_order_then_empty() {
    use passbolt_app::models::ipc_message::IpcMessage;

    let state = AppState::new();

    state.ipc.push_pending_message(IpcMessage::new("topic-1", None));
    state.ipc.push_pending_message(IpcMessage::new("topic-2", None));

    let drained = state.ipc.drain_pending_messages();
    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].topic, "topic-1");
    assert_eq!(drained[1].topic, "topic-2");

    assert_eq!(state.ipc.drain_pending_messages().len(), 0);
}

// ── Auth state ──────────────────────────────────────────────────

/// The passphrase is a one-shot value: `take` returns it once,
/// then returns None. It must never persist in Rust memory
/// after delivery to the workspace Background.
#[test]
fn passphrase_is_consumed_on_take() {
    let state = AppState::new();

    assert!(state.auth.take_passphrase().is_none());

    state.auth.set_passphrase("my-secret".to_string());
    assert_eq!(state.auth.take_passphrase(), Some("my-secret".to_string()));
    assert!(state.auth.take_passphrase().is_none());
}

// ── Navigation state ────────────────────────────────────────────

/// Mode transitions cycle through Import → Auth → Workspace.
/// The initial mode is Import (first launch, no account).
#[test]
fn mode_transitions_through_all_modes() {
    let state = AppState::new();

    assert_eq!(state.navigation.get_mode(), AppMode::Import);

    state.navigation.set_mode(AppMode::Auth);
    assert_eq!(state.navigation.get_mode(), AppMode::Auth);

    state.navigation.set_mode(AppMode::Workspace);
    assert_eq!(state.navigation.get_mode(), AppMode::Workspace);
}

/// The trusted domain is None until an account is imported or loaded
/// from the keyring. Once set, it locks the app to that server.
/// `clear` resets it (used on import rollback).
#[test]
fn trusted_domain_lifecycle() {
    let state = AppState::new();

    assert!(state.navigation.get_trusted_domain().is_none());

    state.navigation.set_trusted_domain("https://passbolt.example.com".to_string());
    assert_eq!(
        state.navigation.get_trusted_domain(),
        Some("https://passbolt.example.com".to_string())
    );

    state.navigation.clear_trusted_domain();
    assert!(state.navigation.get_trusted_domain().is_none());
}

/// `reset_for_navigation` clears both ready flags and the pending message
/// buffer. Called on every mode transition to ensure a clean slate before
/// the new bundles load and the ready handshake replays.
#[test]
fn reset_for_navigation_clears_ready_flags_and_pending_messages() {
    use passbolt_app::models::ipc_message::IpcMessage;

    let state = AppState::new();

    state.navigation.set_background_ready(true);
    state.navigation.set_rendered_ready(true);
    state.ipc.push_pending_message(IpcMessage::new("buffered", None));

    state.reset_for_navigation();

    assert!(!state.navigation.is_background_ready());
    assert!(!state.navigation.is_rendered_ready());
    assert_eq!(state.ipc.drain_pending_messages().len(), 0);
}
