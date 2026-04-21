use serde::{Deserialize, Serialize};

/// Private account material persisted in the OS keyring as `"account-secret"`.
///
/// The armored private key is still passphrase-encrypted — it can only be
/// unlocked by the user's passphrase inside the Background JS runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountSecret {
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "user_private_armored_key")]
    pub user_private_armored_key: String,
}
