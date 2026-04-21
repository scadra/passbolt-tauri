use passbolt_app::models::ipc_message::IpcMessage;

/// `IpcMessage::new` should produce a minimal envelope with only the topic set.
/// All optional fields (status, message, requestId) should be None.
#[test]
fn new_message_has_only_topic() {
    let msg = IpcMessage::new("passbolt.auth.login", None);
    assert_eq!(msg.topic, "passbolt.auth.login");
    assert!(msg.status.is_none());
    assert!(msg.message.is_none());
    assert!(msg.request_id.is_none());
    assert!(!msg.with_multiple_param);
}

/// `IpcMessage::success` creates a response with status "SUCCESS"
/// and the requestId as both `topic` and `request_id` — this is how
/// the JS side correlates responses to their originating Promise.
#[test]
fn success_response_carries_request_id_as_topic() {
    let msg = IpcMessage::success("req-123", Some(serde_json::json!({"ok": true})));
    assert_eq!(msg.topic, "req-123");
    assert_eq!(msg.status, Some("SUCCESS".to_string()));
    assert_eq!(msg.request_id, Some("req-123".to_string()));
}

/// `IpcMessage::error` creates a response with status "ERROR".
/// The JS side rejects the matching Promise on receiving this.
#[test]
fn error_response_has_error_status() {
    let msg = IpcMessage::error("req-456", Some(serde_json::json!("something failed")));
    assert_eq!(msg.topic, "req-456");
    assert_eq!(msg.status, Some("ERROR".to_string()));
}

/// JS sends `requestId` in camelCase. The Rust struct uses `request_id`
/// with a `#[serde(alias = "requestId")]` so both forms deserialize correctly.
/// Same for `withMultipleParam`.
#[test]
fn deserializes_camel_case_fields_from_js() {
    let json = r#"{
        "topic": "passbolt.resource.find-all",
        "requestId": "8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a55",
        "message": ["arg1", "arg2"],
        "withMultipleParam": true
    }"#;

    let msg: IpcMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.topic, "passbolt.resource.find-all");
    assert_eq!(msg.request_id, Some("8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a55".to_string()));
    assert!(msg.with_multiple_param);
    assert!(msg.message.is_some());
}

/// The simplest possible message from JS: just `{ "topic": "..." }`.
/// All other fields should default to None/false, not fail deserialization.
#[test]
fn deserializes_minimal_payload_without_optional_fields() {
    let json = r#"{"topic": "passbolt.background.is-ready"}"#;

    let msg: IpcMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.topic, "passbolt.background.is-ready");
    assert!(msg.status.is_none());
    assert!(msg.request_id.is_none());
    assert!(!msg.with_multiple_param);
}

/// When serializing back to JSON (Rust → JS), None fields should be
/// omitted entirely — not serialized as `"status": null`. This keeps
/// the wire format clean and matches what the JS IPCHandler expects.
#[test]
fn serialization_omits_none_fields() {
    let msg = IpcMessage::new("passbolt.background.is-ready", None);
    let json = serde_json::to_string(&msg).unwrap();

    assert!(json.contains("topic"));
    assert!(!json.contains("status"));
    assert!(!json.contains("message"));
}
