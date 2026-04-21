use passbolt_app::models::ipc_message::IpcMessage;
use passbolt_app::state::{AppMode, AppState};

// ── Ready handshake ─────────────────────────────────────────────

/// `BACKGROUND_READY` sets the background_ready flag so Rust knows
/// it can start forwarding messages and navigating Rendered.
#[test]
fn background_ready_sets_flag() {
    let state = AppState::new();
    assert!(!state.navigation.is_background_ready());

    state.navigation.set_background_ready(true);
    assert!(state.navigation.is_background_ready());
}

/// When a passphrase was cached during login (AFTER_LOGIN), the
/// BACKGROUND_READY handler delivers it to the workspace Background
/// via `take_passphrase` (one-shot consumption).
#[test]
fn background_ready_delivers_cached_passphrase() {
    let state = AppState::new();

    state.auth.set_passphrase("my-passphrase".to_string());

    let passphrase = state.auth.take_passphrase();
    assert_eq!(passphrase, Some("my-passphrase".to_string()));
    assert!(state.auth.take_passphrase().is_none());
}

/// If no login happened before BACKGROUND_READY (e.g. import mode),
/// there is no passphrase to deliver — `take` returns None.
#[test]
fn background_ready_without_passphrase_is_noop() {
    let state = AppState::new();
    assert!(state.auth.take_passphrase().is_none());
}

// ── Auth flow ───────────────────────────────────────────────────

/// AFTER_LOGIN carries the passphrase as a JSON string in `message`.
/// The handler extracts and caches it for the 2-hop relay.
#[test]
fn after_login_extracts_passphrase_from_message() {
    let state = AppState::new();

    let ipc = IpcMessage {
        topic: "passbolt.auth.after-login".to_string(),
        status: None,
        message: Some(serde_json::json!("the-passphrase")),
        request_id: None,
        with_multiple_param: false,
    };

    if let Some(passphrase) = ipc.message.as_ref().and_then(|m| m.as_str()) {
        state.auth.set_passphrase(passphrase.to_string());
    }

    assert_eq!(state.auth.take_passphrase(), Some("the-passphrase".to_string()));
}

/// After login, `redirect_to_workspace` resets all ready flags
/// and switches the mode. Background will be navigated to
/// `index-workspace.html`, and Rendered waits for BACKGROUND_READY.
#[test]
fn after_login_resets_state_and_switches_to_workspace() {
    let state = AppState::new();
    state.navigation.set_mode(AppMode::Auth);
    state.navigation.set_background_ready(true);
    state.navigation.set_rendered_ready(true);

    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Workspace);

    assert_eq!(state.navigation.get_mode(), AppMode::Workspace);
    assert!(!state.navigation.is_background_ready());
    assert!(!state.navigation.is_rendered_ready());
}

/// REQUIRE_MFA carries the passphrase inside a JSON object
/// `{ "provider": "totp", "passphrase": "..." }`.
/// The handler extracts it from the nested field.
#[test]
fn require_mfa_extracts_passphrase_from_nested_payload() {
    let state = AppState::new();

    let ipc = IpcMessage {
        topic: "passbolt.auth.redirect-to-mfa".to_string(),
        status: None,
        message: Some(serde_json::json!({
            "provider": "totp",
            "passphrase": "mfa-passphrase"
        })),
        request_id: None,
        with_multiple_param: false,
    };

    if let Some(msg) = &ipc.message {
        if let Some(passphrase) = msg.get("passphrase").and_then(|p| p.as_str()) {
            state.auth.set_passphrase(passphrase.to_string());
        }
    }

    assert_eq!(state.auth.take_passphrase(), Some("mfa-passphrase".to_string()));
}

/// LOG_OUT tears down the workspace: resets ready flags, clears
/// pending messages, and switches back to Auth mode.
#[test]
fn logout_resets_state_and_switches_to_auth() {
    let state = AppState::new();
    state.navigation.set_mode(AppMode::Workspace);
    state.navigation.set_background_ready(true);
    state.navigation.set_rendered_ready(true);
    state.ipc.push_pending_message(IpcMessage::new("buffered", None));

    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Auth);

    assert_eq!(state.navigation.get_mode(), AppMode::Auth);
    assert!(!state.navigation.is_background_ready());
    assert!(!state.navigation.is_rendered_ready());
    assert_eq!(state.ipc.drain_pending_messages().len(), 0);
}

// ── Response forwarding ─────────────────────────────────────────

/// When a response arrives (topic = previously registered requestId),
/// `proceed_request_id` consumes it (one-shot) to prevent replay.
#[test]
fn response_forward_consumes_request_id() {
    let state = AppState::new();

    state.ipc.add_request_id("req-abc");

    assert!(state.ipc.proceed_request_id("req-abc"));
    assert!(!state.ipc.proceed_request_id("req-abc"));
}

/// When a response matches a pending_request (e.g. RBAC), the
/// original topic is recovered for post-processing, then cleaned up.
#[test]
fn response_forward_with_rbac_interception() {
    let state = AppState::new();

    state.ipc.add_request_id("req-rbac");
    state.ipc.add_pending_request("req-rbac", "passbolt.rbacs.find-me");

    assert!(state.ipc.proceed_request_id("req-rbac"));
    let original_topic = state.ipc.get_pending_request("req-rbac");
    assert_eq!(original_topic, Some("passbolt.rbacs.find-me".to_string()));

    state.ipc.remove_pending_request("req-rbac");
    assert!(!state.ipc.has_pending_request("req-rbac"));
}

/// When a message doesn't match any registered requestId and Rendered
/// is not ready, it gets buffered in pending_messages for later delivery.
#[test]
fn unknown_response_buffered_when_rendered_not_ready() {
    let state = AppState::new();

    let ipc = IpcMessage::new("unknown-topic", None);

    if !state.navigation.is_rendered_ready() {
        state.ipc.push_pending_message(ipc);
    }

    assert_eq!(state.ipc.drain_pending_messages().len(), 1);
}

// ── Full passphrase 2-hop lifecycle ─────────────────────────────

/// Complete passphrase relay through the auth→workspace transition:
///
/// 1. Auth-Background validates passphrase → sends AFTER_LOGIN → Rust caches it
/// 2. Rust resets state, switches to Workspace mode
/// 3. Workspace-Background loads → sends BACKGROUND_READY
/// 4. Rust delivers passphrase to workspace-Background (one-shot)
/// 5. Passphrase is gone from Rust memory
#[test]
fn passphrase_two_hop_relay_lifecycle() {
    let state = AppState::new();

    // Step 1: AFTER_LOGIN
    state.auth.set_passphrase("user-secret".to_string());
    state.navigation.set_mode(AppMode::Auth);

    // Step 2: redirect to workspace
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Workspace);

    // Step 3: BACKGROUND_READY
    state.navigation.set_background_ready(true);

    // Step 4: deliver passphrase
    let passphrase = state.auth.take_passphrase();
    assert_eq!(passphrase, Some("user-secret".to_string()));

    // Step 5: gone
    assert!(state.auth.take_passphrase().is_none());
}
