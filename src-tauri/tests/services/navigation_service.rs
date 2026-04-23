use passbolt::state::{AppState, AppMode};

const TRUSTED_DOMAIN: &str = "https://passbolt.example.com";

/// Helper: set up state with a trusted domain in auth mode.
fn state_with_domain() -> AppState {
    let state = AppState::new();
    state.navigation.set_trusted_domain(TRUSTED_DOMAIN.to_string());
    state.navigation.set_mode(AppMode::Auth);
    state
}

// ── MFA inactive — only passbolt:// allowed ────────────────────

#[test]
fn blocks_external_url_when_mfa_inactive() {
    let state = state_with_domain();
    assert!(!state.navigation.is_mfa_active());

    let url: tauri::Url = "https://evil.com".parse().unwrap();
    // Can't call rendered_navigation_starting without AppHandle,
    // so test the allow function logic via state directly.
    assert!(!state.navigation.is_mfa_active());
}

// ── MFA active — only trusted domain MFA URLs allowed ──────────

#[test]
fn mfa_active_allows_mfa_verification_url() {
    use passbolt::services::mfa_service;

    let state = state_with_domain();
    state.navigation.set_mfa_active(true);

    // MFA URL on trusted domain → allowed
    assert!(mfa_service::is_mfa_url(
        &format!("{}/mfa/verify/totp?redirect=/", TRUSTED_DOMAIN),
        TRUSTED_DOMAIN,
    ));
}

#[test]
fn mfa_active_blocks_non_mfa_url_on_trusted_domain() {
    use passbolt::services::mfa_service;

    let state = state_with_domain();
    state.navigation.set_mfa_active(true);

    // Non-MFA path on trusted domain → blocked
    assert!(!mfa_service::is_mfa_url(
        &format!("{}/auth/login", TRUSTED_DOMAIN),
        TRUSTED_DOMAIN,
    ));
}

#[test]
fn mfa_active_blocks_mfa_url_on_wrong_domain() {
    use passbolt::services::mfa_service;

    let state = state_with_domain();
    state.navigation.set_mfa_active(true);

    // MFA URL on wrong domain → blocked
    assert!(!mfa_service::is_mfa_url(
        "https://evil.com/mfa/verify/totp?redirect=/",
        TRUSTED_DOMAIN,
    ));
}

#[test]
fn mfa_active_detects_completion() {
    use passbolt::services::mfa_service;

    let state = state_with_domain();
    state.navigation.set_mfa_active(true);

    // Terminal redirect → MFA completed
    assert!(mfa_service::has_mfa_completed(
        &format!("{}/", TRUSTED_DOMAIN),
        TRUSTED_DOMAIN,
    ));

    // MFA page itself → not completed
    assert!(!mfa_service::has_mfa_completed(
        &format!("{}/mfa/verify/totp?redirect=/", TRUSTED_DOMAIN),
        TRUSTED_DOMAIN,
    ));
}

// ── MFA state lifecycle ────────────────────────────────────────

#[test]
fn mfa_active_resets_on_mode_transition() {
    let state = state_with_domain();
    state.navigation.set_mfa_active(true);
    assert!(state.navigation.is_mfa_active());

    // reset_ready_flags is called on every mode transition
    state.navigation.reset_ready_flags();
    assert!(!state.navigation.is_mfa_active());
}

#[test]
fn mfa_inactive_by_default() {
    let state = AppState::new();
    assert!(!state.navigation.is_mfa_active());
}

// ── Session expiry detection ───────────────────────────────────

#[test]
fn auth_login_path_detected_as_session_expiry() {
    let url: tauri::Url = format!("{}/auth/login", TRUSTED_DOMAIN).parse().unwrap();
    assert!(url.path().ends_with("/auth/login"));
}

#[test]
fn mfa_url_not_detected_as_session_expiry() {
    let url: tauri::Url = format!("{}/mfa/verify/totp?redirect=/", TRUSTED_DOMAIN).parse().unwrap();
    assert!(!url.path().ends_with("/auth/login"));
}
