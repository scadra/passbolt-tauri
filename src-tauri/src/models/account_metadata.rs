use serde::{Deserialize, Serialize};

/// Anti-phishing token shown on the login screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityToken {
    pub code: String,
    pub color: String,
    #[serde(rename = "textcolor", alias = "textColor")]
    pub text_color: String,
}

/// Public account information persisted in the OS keyring as `"account-metadata"`.
///
/// Contains everything needed to display the login screen and initialize
/// the passbolt-browser-extension account context — but NOT the private key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountMetadata {
    pub domain: String,
    #[serde(rename = "user_id")]
    pub user_id: String,
    pub username: String,
    #[serde(rename = "first_name")]
    pub first_name: String,
    #[serde(rename = "last_name")]
    pub last_name: String,
    #[serde(rename = "security_token")]
    pub security_token: SecurityToken,
    #[serde(rename = "server_public_armored_key")]
    pub server_public_armored_key: String,
    #[serde(rename = "user_public_armored_key")]
    pub user_public_armored_key: String,
    pub theme: Option<String>,
    pub locale: Option<String>,
}
