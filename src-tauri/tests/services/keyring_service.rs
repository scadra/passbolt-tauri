use passbolt_app::models::account_metadata::{AccountMetadata, SecurityToken};
use passbolt_app::models::account_secret::AccountSecret;
use passbolt_app::services::keyring_service::KeyringService;

/// Helper: create a service with a unique name to avoid test collisions.
fn test_service(name: &str) -> KeyringService {
    KeyringService::new(&format!("com.passbolt.test.{}", name))
}

/// Helper: build a sample AccountMetadata.
fn sample_metadata() -> AccountMetadata {
    AccountMetadata {
        domain: "https://passbolt.example.com".to_string(),
        user_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        username: "ada@passbolt.com".to_string(),
        first_name: "Ada".to_string(),
        last_name: "Lovelace".to_string(),
        security_token: SecurityToken {
            code: "ABC".to_string(),
            color: "#ff0000".to_string(),
            text_color: "#ffffff".to_string(),
        },
        server_public_armored_key: "-----BEGIN PGP PUBLIC KEY-----\nserver\n-----END PGP PUBLIC KEY-----".to_string(),
        user_public_armored_key: "-----BEGIN PGP PUBLIC KEY-----\nuser\n-----END PGP PUBLIC KEY-----".to_string(),
        theme: Some("default".to_string()),
        locale: Some("en-UK".to_string()),
    }
}

/// Helper: build a sample AccountSecret.
fn sample_secret() -> AccountSecret {
    AccountSecret {
        user_id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string(),
        user_private_armored_key: "-----BEGIN PGP PRIVATE KEY-----\narmored\n-----END PGP PRIVATE KEY-----".to_string(),
    }
}

/// Cleanup helper — remove test credentials after each test.
fn cleanup(service: &KeyringService) {
    let _ = service.remove_account();
}

// ── Account existence ───────────────────────────────────────────

/// An empty keyring has no account.
#[test]
fn has_account_returns_false_when_empty() {
    let service = test_service("has_account_empty");
    cleanup(&service);

    assert!(!service.has_account());
}

/// After creating an account, `has_account` returns true.
#[test]
fn create_account_then_has_account_returns_true() {
    let service = test_service("has_account_exists");
    cleanup(&service);

    service.create_account(&sample_metadata(), &sample_secret()).unwrap();
    assert!(service.has_account());

    cleanup(&service);
}

// ── Metadata round-trip ─────────────────────────────────────────

/// Save metadata then read it back — all fields must match.
#[test]
fn get_metadata_returns_saved_data() {
    let service = test_service("metadata_roundtrip");
    cleanup(&service);

    let metadata = sample_metadata();
    service.save_account_metadata(&metadata).unwrap();

    let loaded = service.get_account_metadata().unwrap();
    assert_eq!(loaded.domain, metadata.domain);
    assert_eq!(loaded.user_id, metadata.user_id);
    assert_eq!(loaded.username, metadata.username);
    assert_eq!(loaded.first_name, metadata.first_name);
    assert_eq!(loaded.last_name, metadata.last_name);
    assert_eq!(loaded.security_token, metadata.security_token);
    assert_eq!(loaded.theme, metadata.theme);
    assert_eq!(loaded.locale, metadata.locale);

    cleanup(&service);
}

// ── Secret round-trip ───────────────────────────────────────────

/// Save secret then read it back — all fields must match.
#[test]
fn get_secret_returns_saved_data() {
    let service = test_service("secret_roundtrip");
    cleanup(&service);

    let secret = sample_secret();
    service.save_account_secret(&secret).unwrap();

    let loaded = service.get_account_secret().unwrap();
    assert_eq!(loaded.user_id, secret.user_id);
    assert_eq!(loaded.user_private_armored_key, secret.user_private_armored_key);

    cleanup(&service);
}

// ── Remove ──────────────────────────────────────────────────────

/// After removing an account, both metadata and secret are gone.
#[test]
fn remove_account_clears_all() {
    let service = test_service("remove_account");
    cleanup(&service);

    service.create_account(&sample_metadata(), &sample_secret()).unwrap();
    assert!(service.has_account());

    service.remove_account().unwrap();
    assert!(!service.has_account());
    assert!(service.get_account_secret().is_none());
}

// ── Field updates ───────────────────────────────────────────────

/// Update theme persists to the keyring.
#[test]
fn update_theme_persists() {
    let service = test_service("update_theme");
    cleanup(&service);

    service.save_account_metadata(&sample_metadata()).unwrap();
    service.update_theme("midgar").unwrap();

    let loaded = service.get_account_metadata().unwrap();
    assert_eq!(loaded.theme, Some("midgar".to_string()));

    cleanup(&service);
}

/// Update locale persists to the keyring.
#[test]
fn update_locale_persists() {
    let service = test_service("update_locale");
    cleanup(&service);

    service.save_account_metadata(&sample_metadata()).unwrap();
    service.update_locale("fr-FR").unwrap();

    let loaded = service.get_account_metadata().unwrap();
    assert_eq!(loaded.locale, Some("fr-FR".to_string()));

    cleanup(&service);
}

/// Update security token persists to the keyring.
#[test]
fn update_security_token_persists() {
    let service = test_service("update_token");
    cleanup(&service);

    service.save_account_metadata(&sample_metadata()).unwrap();

    let new_token = SecurityToken {
        code: "XYZ".to_string(),
        color: "#00ff00".to_string(),
        text_color: "#000000".to_string(),
    };
    service.update_security_token(&new_token).unwrap();

    let loaded = service.get_account_metadata().unwrap();
    assert_eq!(loaded.security_token, new_token);

    cleanup(&service);
}

/// Update private key persists to the keyring.
#[test]
fn update_private_key_persists() {
    let service = test_service("update_key");
    cleanup(&service);

    service.save_account_secret(&sample_secret()).unwrap();
    service.update_private_key("-----BEGIN PGP PRIVATE KEY-----\nnew-key\n-----END PGP PRIVATE KEY-----").unwrap();

    let loaded = service.get_account_secret().unwrap();
    assert!(loaded.user_private_armored_key.contains("new-key"));

    cleanup(&service);
}

// ── Overwrite ───────────────────────────────────────────────────

/// A second `create_account` overwrites the first — no multi-account support.
#[test]
fn create_account_overwrites_existing() {
    let service = test_service("overwrite");
    cleanup(&service);

    service.create_account(&sample_metadata(), &sample_secret()).unwrap();

    let mut new_metadata = sample_metadata();
    new_metadata.username = "bob@passbolt.com".to_string();
    let mut new_secret = sample_secret();
    new_secret.user_private_armored_key = "new-armored-key".to_string();

    service.create_account(&new_metadata, &new_secret).unwrap();

    let loaded_meta = service.get_account_metadata().unwrap();
    assert_eq!(loaded_meta.username, "bob@passbolt.com");

    let loaded_secret = service.get_account_secret().unwrap();
    assert_eq!(loaded_secret.user_private_armored_key, "new-armored-key");

    cleanup(&service);
}

// ── Error cases ─────────────────────────────────────────────────

/// Updating theme without an account returns an error.
#[test]
fn update_theme_without_account_returns_error() {
    let service = test_service("update_no_account");
    cleanup(&service);

    let result = service.update_theme("midgar");
    assert!(result.is_err());
}
