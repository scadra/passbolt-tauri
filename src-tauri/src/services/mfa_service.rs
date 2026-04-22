//! MFA URL validation — equivalent of Services/Mfa/MfaService.cs
//!
//! Stateless functions that check whether a URL belongs to the MFA verification
//! flow on a given trusted domain. Used by `navigation_service` to decide
//! whether the Rendered WebView is allowed to navigate to an external URL.

/// Check if the URL is an MFA verification page on the trusted domain.
/// Equivalent of `MfaService.IsMfaUrls(url)`.
pub fn is_mfa_url(url: &str, trusted_domain: &str) -> bool {
    url.starts_with(&format!("{}/mfa/verify/", trusted_domain))
        || has_mfa_completed(url, trusted_domain)
}

/// Check if the URL indicates MFA verification is complete (terminal redirect).
/// The Passbolt API redirects to `{domain}/` after successful MFA verification.
/// Equivalent of `MfaService.HasMfaCompleted(url)`.
pub fn has_mfa_completed(url: &str, trusted_domain: &str) -> bool {
    let root = if trusted_domain.ends_with('/') {
        trusted_domain.to_string()
    } else {
        format!("{}/", trusted_domain)
    };
    url == root || url == trusted_domain
}

/// Build the MFA verification URL for a given provider.
/// Returns `{trusted_domain}/mfa/verify/{provider}?redirect=/`.
pub fn build_mfa_url(trusted_domain: &str, provider: &str) -> String {
    format!("{}/mfa/verify/{}?redirect=/", trusted_domain, provider)
}