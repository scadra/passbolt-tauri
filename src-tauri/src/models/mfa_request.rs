use serde::Deserialize;

/// Payload from the Background webview when MFA is required.
///
/// Contains the user's passphrase (to cache for later workspace transition)
/// and the MFA provider type (totp, yubikey, etc.).
#[derive(Debug, Deserialize)]
pub struct MfaRequest {
    pub passphrase: Option<String>,

    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_provider() -> String {
    "totp".to_string()
}
