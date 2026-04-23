//! URL origin validation — prevents SSRF via prefix-matching bypass.
//!
//! Provides `is_trusted_origin()` to compare two URLs by their parsed host and scheme,
//! rather than naive `starts_with()` string matching which allows
//! `https://trusted.com.attacker.com` to pass as trusted.

/// Check if a candidate URL belongs to a trusted domain by comparing parsed host and scheme.
///
/// Returns `false` if either URL fails to parse, or if host/scheme don't match exactly.
/// This prevents SSRF attacks where `https://trusted.com.attacker.com` would pass a
/// naive `starts_with("https://trusted.com")` check.
pub fn is_trusted_origin(candidate_url: &str, trusted_domain: &str) -> bool {
    let parsed_candidate = match url::Url::parse(candidate_url) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    let parsed_trusted = match url::Url::parse(trusted_domain) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };

    let candidate_host = parsed_candidate.host_str().unwrap_or("");
    let trusted_host = parsed_trusted.host_str().unwrap_or("");
    let candidate_scheme = parsed_candidate.scheme();
    let trusted_scheme = parsed_trusted.scheme();
    let candidate_port = parsed_candidate.port_or_known_default();
    let trusted_port = parsed_trusted.port_or_known_default();

    candidate_scheme == trusted_scheme
        && candidate_host == trusted_host
        && candidate_port == trusted_port
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching_origin() {
        assert!(is_trusted_origin(
            "https://passbolt.example.com/api/users.json",
            "https://passbolt.example.com"
        ));
    }

    #[test]
    fn test_matching_origin_with_trailing_slash() {
        assert!(is_trusted_origin(
            "https://passbolt.example.com/api/users.json",
            "https://passbolt.example.com/"
        ));
    }

    #[test]
    fn test_subdomain_attack_blocked() {
        assert!(!is_trusted_origin(
            "https://passbolt.example.com.attacker.com/api/",
            "https://passbolt.example.com"
        ));
    }

    #[test]
    fn test_scheme_mismatch_blocked() {
        assert!(!is_trusted_origin(
            "http://passbolt.example.com/api/",
            "https://passbolt.example.com"
        ));
    }

    #[test]
    fn test_different_host_blocked() {
        assert!(!is_trusted_origin(
            "https://other-domain.com/api/",
            "https://passbolt.example.com"
        ));
    }

    #[test]
    fn test_malformed_candidate_returns_false() {
        assert!(!is_trusted_origin(
            "not-a-valid-url",
            "https://passbolt.example.com"
        ));
    }

    #[test]
    fn test_malformed_trusted_returns_false() {
        assert!(!is_trusted_origin(
            "https://passbolt.example.com/api/",
            "not-a-valid-url"
        ));
    }

    #[test]
    fn test_empty_strings_return_false() {
        assert!(!is_trusted_origin("", ""));
        assert!(!is_trusted_origin("https://example.com", ""));
        assert!(!is_trusted_origin("", "https://example.com"));
    }

    #[test]
    fn test_pwnedpasswords_origin() {
        assert!(is_trusted_origin(
            "https://api.pwnedpasswords.com/range/5BAA6",
            "https://api.pwnedpasswords.com"
        ));
    }

    #[test]
    fn test_pwnedpasswords_subdomain_attack_blocked() {
        assert!(!is_trusted_origin(
            "https://api.pwnedpasswords.com.evil.com/range/5BAA6",
            "https://api.pwnedpasswords.com"
        ));
    }

    #[test]
    fn test_port_mismatch_blocked() {
        assert!(!is_trusted_origin(
            "https://passbolt.example.com:8443/api/",
            "https://passbolt.example.com"
        ));
    }
}
