use passbolt_app::services::mfa_service;

const DOMAIN: &str = "https://passbolt.example.com";

#[test]
fn is_mfa_url_matches_totp() {
    assert!(mfa_service::is_mfa_url("https://passbolt.example.com/mfa/verify/totp?redirect=/", DOMAIN));
}

#[test]
fn is_mfa_url_matches_yubikey() {
    assert!(mfa_service::is_mfa_url("https://passbolt.example.com/mfa/verify/yubikey?redirect=/", DOMAIN));
}

#[test]
fn is_mfa_url_matches_duo() {
    assert!(mfa_service::is_mfa_url("https://passbolt.example.com/mfa/verify/duo?redirect=/", DOMAIN));
}

#[test]
fn is_mfa_url_matches_terminal_redirect() {
    assert!(mfa_service::is_mfa_url("https://passbolt.example.com/", DOMAIN));
}

#[test]
fn is_mfa_url_rejects_other_domains() {
    assert!(!mfa_service::is_mfa_url("https://evil.com/mfa/verify/totp?redirect=/", DOMAIN));
}

#[test]
fn is_mfa_url_rejects_non_mfa_paths() {
    assert!(!mfa_service::is_mfa_url("https://passbolt.example.com/auth/login", DOMAIN));
}

#[test]
fn has_mfa_completed_detects_root_with_trailing_slash() {
    assert!(mfa_service::has_mfa_completed("https://passbolt.example.com/", DOMAIN));
}

#[test]
fn has_mfa_completed_detects_root_without_trailing_slash() {
    assert!(mfa_service::has_mfa_completed("https://passbolt.example.com", DOMAIN));
}

#[test]
fn has_mfa_completed_rejects_mfa_page() {
    assert!(!mfa_service::has_mfa_completed("https://passbolt.example.com/mfa/verify/totp?redirect=/", DOMAIN));
}

#[test]
fn build_mfa_url_formats_correctly() {
    assert_eq!(
        mfa_service::build_mfa_url(DOMAIN, "totp"),
        "https://passbolt.example.com/mfa/verify/totp?redirect=/"
    );
}
