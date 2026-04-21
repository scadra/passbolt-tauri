use passbolt_app::services::rendered_topic_service::is_valid_uuid;

/// Standard UUID v4 with lowercase hex — the format JS `uuidv4()` produces.
#[test]
fn accepts_valid_uuid_v4() {
    assert!(is_valid_uuid("8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a55"));
}

/// All-zeros UUID is valid (used as a sentinel in some RBAC entries).
#[test]
fn accepts_all_zeros_uuid() {
    assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));
}

/// Random strings must not pass — prevents topic injection.
#[test]
fn rejects_random_string() {
    assert!(!is_valid_uuid("not-a-uuid"));
}

/// Empty string is not a UUID.
#[test]
fn rejects_empty_string() {
    assert!(!is_valid_uuid(""));
}

/// One character short — catches off-by-one in the regex.
#[test]
fn rejects_truncated_uuid() {
    assert!(!is_valid_uuid("8a7d7f1b-9e32-4d5c-8d21-3b6c9a2f1a5"));
}

/// Uppercase hex is rejected — the IPC protocol uses lowercase only.
/// Accepting uppercase would widen the attack surface for no benefit.
#[test]
fn rejects_uppercase_uuid() {
    assert!(!is_valid_uuid("8A7D7F1B-9E32-4D5C-8D21-3B6C9A2F1A55"));
}
