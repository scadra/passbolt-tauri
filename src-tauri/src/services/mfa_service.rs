//! MFA URL validation — equivalent of Services/Mfa/MfaService.cs
//!
//! Stateless functions that check whether a URL belongs to the MFA verification
//! flow on a given trusted domain. Used by `navigation_service` to decide
//! whether the Rendered WebView is allowed to navigate to an external URL.

/// Check if the URL is an MFA verification page on the trusted domain.
/// Equivalent of `MfaService.IsMfaUrls(url)`.
///
/// Uses parsed URL host comparison instead of string prefix matching
/// to prevent bypass via `https://trusted.com.attacker.com/mfa/verify/`.
pub fn is_mfa_url(url: &str, trusted_domain: &str) -> bool {
    if !super::url_validation::is_trusted_origin(url, trusted_domain) {
        return false;
    }
    let parsed_url = match url::Url::parse(url) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    parsed_url.path().starts_with("/mfa/verify/") || has_mfa_completed(url, trusted_domain)
}

/// Check if the URL indicates MFA verification is complete (terminal redirect).
/// The Passbolt API redirects to `{domain}/` after successful MFA verification.
/// Equivalent of `MfaService.HasMfaCompleted(url)`.
///
/// Uses parsed URL host comparison instead of string equality to be resilient
/// to trailing slash variations and query parameter additions.
pub fn has_mfa_completed(url: &str, trusted_domain: &str) -> bool {
    if !super::url_validation::is_trusted_origin(url, trusted_domain) {
        return false;
    }
    let parsed_url = match url::Url::parse(url) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };
    // MFA completion is indicated by a redirect to the domain root
    parsed_url.path() == "/" || parsed_url.path().is_empty()
}

/// Build the MFA verification URL for a given provider.
/// Returns `{trusted_domain}/mfa/verify/{provider}?redirect=/`.
pub fn build_mfa_url(trusted_domain: &str, provider: &str) -> String {
    format!("{}/mfa/verify/{}?redirect=/", trusted_domain, provider)
}