use passbolt_app::models::messaging::AllowedTopics;

/// Topics defined in the 33 topic modules (e.g. `AuthenticationTopics::DESKTOP_AUTHENTICATE`)
/// must be present in the whitelist after `AllowedTopics::new()` aggregates them.
#[test]
fn registered_topics_from_modules_are_allowed() {
    let topics = AllowedTopics::new();

    assert!(topics.is_allowed("passbolt.auth.login"));
    assert!(topics.is_allowed("passbolt.resources.find-details"));
    assert!(topics.is_allowed("passbolt.rbacs.find-me"));
    assert!(topics.is_allowed("passbolt.secret.find-by-resource-id"));
}

/// Native-handled topics (defined in `config.rs`, not in topic modules)
/// are added separately. They must also pass validation because
/// Background emits them directly (e.g. `BACKGROUND_READY`).
#[test]
fn native_topics_from_config_are_allowed() {
    let topics = AllowedTopics::new();

    assert!(topics.is_allowed("passbolt.background.is-ready"));
    assert!(topics.is_allowed("passbolt.rendered.is-ready"));
    assert!(topics.is_allowed("passbolt.background.store-passphrase"));
    assert!(topics.is_allowed("passbolt.background.set-theme"));
    assert!(topics.is_allowed("passbolt.background.download-file"));
}

/// Any topic not in the registry must be rejected.
/// This prevents a compromised script from invoking arbitrary handlers.
#[test]
fn unknown_topics_are_rejected() {
    let topics = AllowedTopics::new();

    assert!(!topics.is_allowed("passbolt.evil.steal-data"));
    assert!(!topics.is_allowed(""));
    assert!(!topics.is_allowed("random-string"));
}
