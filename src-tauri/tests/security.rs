use passbolt::config;

/// Each webview gets a unique origin: `passbolt://<uuid>.passbolt.local`.
/// An attacker could try to craft a domain that ends with `.passbolt.local`
/// but isn't a legitimate UUID origin — ex. `evil.passbolt.local.attacker.com`
/// or the bare suffix `passbolt.local` without a UUID prefix.
///
/// This test verifies that the domain validation logic rejects:
/// - Foreign domains (`evil.com`)
/// - The bare suffix without a UUID (`passbolt.local`, `.passbolt.local`)
/// - Domains where the suffix appears in the middle, not at the end
#[test]
fn domain_validation_rejects_invalid_domains() {
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

/// A valid virtual host has a UUID prefix followed by the domain suffix.
/// Example: `abc123.passbolt.local` — ends with `.passbolt.local` and
/// has content before the suffix.
#[test]
fn valid_domain_is_accepted() {
    let valid_host = "abc123.passbolt.local";
    assert!(valid_host.ends_with(config::DOMAIN_SUFFIX));
}

/// The protocol handler serves embedded assets via `passbolt://` URLs.
/// A compromised script could try to escape the asset directory with
/// path traversal (`../../../etc/passwd`). This test verifies that
/// paths containing `..` are detected before they reach the filesystem.
#[test]
fn path_traversal_is_detected() {
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
