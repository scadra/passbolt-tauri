use passbolt_app::services::ipc_listener_service::parse_ipc_payload;

/// A valid IPC JSON payload is parsed into an IpcMessage.
#[test]
fn parses_valid_ipc_payload() {
    let payload = r#"{"topic": "passbolt.auth.login", "requestId": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"}"#;
    let result = parse_ipc_payload(payload);
    assert!(result.is_some());

    let ipc = result.unwrap().unwrap();
    assert_eq!(ipc.topic, "passbolt.auth.login");
    assert_eq!(ipc.request_id, Some("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee".to_string()));
}

/// A minimal payload with only a topic field is valid.
#[test]
fn parses_minimal_payload() {
    let payload = r#"{"topic": "passbolt.background.is-ready"}"#;
    let ipc = parse_ipc_payload(payload).unwrap().unwrap();
    assert_eq!(ipc.topic, "passbolt.background.is-ready");
    assert!(ipc.status.is_none());
    assert!(ipc.request_id.is_none());
}

/// A payload with all fields populated is parsed correctly.
#[test]
fn parses_full_payload() {
    let payload = r#"{
        "topic": "passbolt.resource.find-all",
        "status": "SUCCESS",
        "message": {"resources": []},
        "requestId": "11111111-2222-3333-4444-555555555555",
        "withMultipleParam": false
    }"#;
    let ipc = parse_ipc_payload(payload).unwrap().unwrap();
    assert_eq!(ipc.topic, "passbolt.resource.find-all");
    assert_eq!(ipc.status, Some("SUCCESS".to_string()));
    assert!(ipc.message.is_some());
    assert!(!ipc.with_multiple_param);
}

/// An empty string returns None (JS emits without data).
#[test]
fn rejects_empty_string() {
    assert!(parse_ipc_payload("").is_none());
}

/// A JSON object without a `topic` field is invalid — parsing returns an error.
#[test]
fn rejects_payload_without_topic() {
    let payload = r#"{"status": "SUCCESS"}"#;
    assert!(parse_ipc_payload(payload).unwrap().is_err());
}

/// Completely invalid JSON is rejected.
#[test]
fn rejects_invalid_json() {
    assert!(parse_ipc_payload("not json at all").unwrap().is_err());
}

/// A JSON array (instead of an object) is rejected.
#[test]
fn rejects_json_array() {
    assert!(parse_ipc_payload(r#"[{"topic": "test"}]"#).unwrap().is_err());
}
