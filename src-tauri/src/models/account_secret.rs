use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Private account material persisted in the OS keyring as `"account-secret"`.
///
/// The armored private key is still passphrase-encrypted — it can only be
/// unlocked by the user's passphrase inside the Background JS runtime.
///
/// Implements `ZeroizeOnDrop` so the private key is scrubbed from memory
/// when the value is dropped (including when `Option<AccountSecret>` is
/// set to `None`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Zeroize, ZeroizeOnDrop)]
pub struct AccountSecret {
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "user_private_armored_key")]
    pub user_private_armored_key: String,
}
