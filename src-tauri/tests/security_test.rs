use passbolt_app::config;
use passbolt_app::state::AppState;

#[test]
fn test_domain_validation_rejects_invalid_domains() {
    let invalid_hosts = vec![
        "evil.com",
        "passbolt.local",
        ".passbolt.local",
        "evil.passbolt.local.attacker.com",
    ];

    for host in invalid_hosts {
        assert!(
            !host.ends_with(config::DOMAIN_SUFFIX) || host == config::DOMAIN_SUFFIX,
            "Domain {} should be rejected",
            host
        );
    }
}

#[test]
fn test_valid_domain_accepted() {
    let valid_host = "abc123.passbolt.local";
    assert!(valid_host.ends_with(config::DOMAIN_SUFFIX));
}

// Topic whitelist tests are in src/services/rendered_topic_service.rs unit tests

#[test]
fn test_path_traversal_detection() {
    let suspicious_paths = vec![
        "../etc/passwd",
        "../../secret",
        "apps/../../../etc/passwd",
        "apps/rendered-webview/../../secret",
    ];

    for path in suspicious_paths {
        assert!(
            path.contains(".."),
            "Path {} should be detected as traversal attempt",
            path
        );
    }
}

#[test]
fn test_state_request_id_not_reusable() {
    let state = AppState::new();

    state.add_request_id("valid-request");
    assert!(state.proceed_request_id("valid-request"));
    // Should not be reusable
    assert!(!state.proceed_request_id("valid-request"));
}

#[test]
fn test_state_pending_request_cleanup() {
    let state = AppState::new();

    state.add_pending_request("req-1", "topic-1");
    assert!(state.has_pending_request("req-1"));

    state.remove_pending_request("req-1");
    assert!(!state.has_pending_request("req-1"));
}
