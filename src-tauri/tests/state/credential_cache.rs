use passbolt_app::models::account_metadata::{AccountMetadata, SecurityToken};
use passbolt_app::models::account_secret::AccountSecret;
use passbolt_app::state::AppState;

fn test_metadata() -> AccountMetadata {
    AccountMetadata {
        domain: "https://example.com".into(),
        user_id: "uuid-1".into(),
        username: "ada@example.com".into(),
        first_name: "Ada".into(),
        last_name: "Lovelace".into(),
        security_token: SecurityToken {
            code: "ABC".into(),
            color: "#fff".into(),
            text_color: "#000".into(),
        },
        server_public_armored_key: "server-key".into(),
        user_public_armored_key: "user-key".into(),
        theme: None,
        locale: None,
    }
}

fn test_secret() -> AccountSecret {
    AccountSecret {
        user_id: "uuid-1".into(),
        user_private_armored_key: "-----BEGIN PGP PRIVATE KEY-----\nSENSITIVE\n-----END PGP PRIVATE KEY-----".into(),
    }
}

#[test]
fn get_before_set_returns_none() {
    let state = AppState::new();
    assert!(state.credentials.get_metadata().is_none());
    assert!(state.credentials.get_secret().is_none());
}

#[test]
fn set_then_get_returns_value() {
    let state = AppState::new();
    state.credentials.set_metadata(test_metadata());
    state.credentials.set_secret(test_secret());
    assert_eq!(state.credentials.get_metadata().unwrap().username, "ada@example.com");
    assert_eq!(state.credentials.get_secret().unwrap().user_id, "uuid-1");
}

#[test]
fn clear_resets_both_atomically() {
    let state = AppState::new();
    state.credentials.set_metadata(test_metadata());
    state.credentials.set_secret(test_secret());
    state.credentials.clear();
    assert!(state.credentials.get_metadata().is_none());
    assert!(state.credentials.get_secret().is_none());
}

#[test]
fn set_overwrites_previous() {
    let state = AppState::new();
    state.credentials.set_secret(test_secret());
    let mut new_secret = test_secret();
    new_secret.user_id = "uuid-2".into();
    state.credentials.set_secret(new_secret);
    assert_eq!(state.credentials.get_secret().unwrap().user_id, "uuid-2");
}

#[test]
fn with_secret_accesses_without_clone() {
    let state = AppState::new();
    state.credentials.set_secret(test_secret());
    let user_id = state.credentials.with_secret(|secret| {
        secret.map(|s| s.user_id.clone())
    });
    assert_eq!(user_id.unwrap(), "uuid-1");
}
