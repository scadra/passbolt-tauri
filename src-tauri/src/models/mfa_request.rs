use serde::Deserialize;
use zeroize::Zeroizing;

/// Payload from the Background webview when MFA is required.
///
/// Contains the user's passphrase (to cache for later workspace transition)
/// and the MFA provider type (totp, yubikey, etc.).
///
/// `Zeroizing<String>` ensures the passphrase bytes are wiped when this
/// struct is dropped, preventing it lingering in memory after use.
#[derive(Debug, Deserialize)]
pub struct MfaRequest {
    pub passphrase: Option<Zeroizing<String>>,

    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_provider() -> String {
    "totp".to_string()
}
