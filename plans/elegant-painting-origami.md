# Topic Comparison: passbolt-windows vs passbolt-desktops (Rust/Tauri)

## Context
Cross-validation of IPC topic whitelists between the Windows C# project and the Tauri Rust project to identify missing topics, stale topics, and bugs.

---

## 1. Topics in Windows but MISSING from Rust

These topics exist in `passbolt-windows` but are **not registered** in the Tauri project (neither in `src-tauri/src/topics/*.rs` modules nor in `src-tauri/src/config.rs` native topics, nor in `src-tauri/src/models/messaging.rs` AllowedTopics):

| # | Topic string | Windows location | Notes |
|---|---|---|---|
| 1 | `passbolt.error` | `AllowedTopics.cs` (ERROR) | Core error topic, used for generic error forwarding |
| 2 | `passbolt.background.import` | `AllowedTopics.cs` (BACKGROUND_IMPORT) | Triggers background import mode. Also used in JS (`appEventEnumeration.js`) |
| 3 | `passbolt.auth.after-logout` | `AllowedTopics.cs` (BACKGROUND_AFTER_LOGOUT) | Post-logout cleanup event. Also used in JS |
| 4 | `passbolt.tab.reload` | `AllowedTopics.cs` (RENDERED_RELOAD) | Triggers webview reload |
| 5 | `passbolt.background.clipboard-set-text` | `AllowedTopics.cs` (BACKGROUND_CLIPBOARD_SET_TEXT) | Clipboard write (different from `passbolt.clipboard.copy` which Rust does have) |
| 6 | `passbolt.account-recovery.has-user-postponed-user-setting-invitation` | `AccountRecoveryTopics.cs` (HAS_POSTPONED_USER_SETTING_INVITATION) | Missing from Rust `account_recovery_topics.rs` |
| 7 | `passbolt.export-policies.get` | `ImportExportTopics.cs` (EXPORT_POLICIES_GET) | Missing from Rust `import_export_topics.rs` |

---

## 2. Topics in Rust but NOT in Windows

These topics exist in the Tauri project but **not** in `passbolt-windows`:

| # | Topic string | Rust location | Notes |
|---|---|---|---|
| 1 | `passbolt.background.listener-ready` | `config.rs` (BACKGROUND_LISTENER_READY) | Tauri-specific: signals background JS listener is attached |
| 2 | `passbolt.account.set-current` | `config.rs` (ACCOUNT_SET_CURRENT) | Set current account in state |
| 3 | `passbolt.http.fetch` | `config.rs` (BACKGROUND_HTTP_FETCH) | HTTP fetch proxy for background webview |

---

## 3. Topics used in JS but NOT in Rust AllowedTopics

These are emitted/listened by the JS webviews but will be **rejected** by Rust's `AllowedTopics::is_allowed()`:

| # | Topic string | JS usage location | Notes |
|---|---|---|---|
| 1 | `passbolt.desktop.authenticate` | `apps/rendered-webview/` | Desktop-specific auth flow, not in any Rust topic module |
| 2 | `passbolt.keyring.get-private-key` | `apps/background-webview/` | Not in Rust `keyring_topics.rs` (which has `download-my-private-key` but not `get-private-key`) |
| 3 | `passbolt.users.update-avatar` | `apps/rendered-webview/` | Not in Rust `user_topics.rs` |

---

## 4. Bug: `passbolt.http.fetch` not in AllowedTopics

**File:** `src-tauri/src/models/messaging.rs:58-73`

`BACKGROUND_HTTP_FETCH` is defined in `config.rs` (line 32) but is **not added** to the native topics vector in `AllowedTopics::new()`. The native list has 14 entries but omits `BACKGROUND_HTTP_FETCH`. Any message with topic `passbolt.http.fetch` will fail validation.

---

## 5. Topics that MATCH (no action needed)

All 33 topic modules have identical string values between Windows and Rust:
- `account_topics` (1 topic)
- `action_logs_topics` (1)
- `auth_import_topics` (5)
- `authentication_topics` (10)
- `clipboard_topics` (3)
- `comment_topics` (3)
- `favorite_topics` (2)
- `folder_topics` (8)
- `group_topics` (7)
- `import_export_topics` (2 of 3 - missing `export-policies.get`)
- `keyring_topics` (5)
- `locale_storage_topics` (6)
- `locale_topics` (2)
- `metadata_topics` (3)
- `mfa_topics` (10)
- `password_expiry_topics` (2)
- `password_generator_topics` (1)
- `password_policies_topics` (4)
- `permission_topics` (1)
- `powned_password_topics` (1)
- `progress_topics` (4)
- `rbac_topics` (1)
- `resource_topics` (11)
- `resource_type_topics` (1)
- `role_topics` (2)
- `secret_history_topics` (4)
- `secret_topics` (2)
- `settings_topics` (3)
- `share_topics` (4)
- `tag_topics` (5)
- `theme_topics` (2)
- `user_topics` (10)

---

## 6. Recommended Actions

### A. Add missing topics from Windows to Rust

**`src-tauri/src/config.rs`** - Add native constants:
```rust
pub const ERROR: &str = "passbolt.error";
pub const BACKGROUND_IMPORT: &str = "passbolt.background.import";
pub const BACKGROUND_AFTER_LOGOUT: &str = "passbolt.auth.after-logout";
pub const RENDERED_RELOAD: &str = "passbolt.tab.reload";
pub const BACKGROUND_CLIPBOARD_SET_TEXT: &str = "passbolt.background.clipboard-set-text";
```

**`src-tauri/src/topics/account_recovery_topics.rs`** - Add:
```
HAS_POSTPONED_USER_SETTING_INVITATION => "passbolt.account-recovery.has-user-postponed-user-setting-invitation"
```

**`src-tauri/src/topics/import_export_topics.rs`** - Add:
```
EXPORT_POLICIES_GET => "passbolt.export-policies.get"
```

### B. Fix AllowedTopics bug

**`src-tauri/src/models/messaging.rs`** - Add `BACKGROUND_HTTP_FETCH` to the native vector:
```rust
crate::config::BACKGROUND_HTTP_FETCH,
```

And add the other new native constants (ERROR, BACKGROUND_IMPORT, etc.) to the same vector.

### C. Add JS-used topics to Rust

Either add to topic modules or config.rs:
- `passbolt.desktop.authenticate` - likely needs a new topic or goes in `authentication_topics.rs`
- `passbolt.keyring.get-private-key` - add to `keyring_topics.rs`
- `passbolt.users.update-avatar` - add to `user_topics.rs`

### D. Evaluate Rust-only topics

The 3 Rust-only topics (`listener-ready`, `account.set-current`, `http.fetch`) appear intentional for the Tauri architecture. No action needed unless Windows needs them too.

---

## Verification

After changes:
```bash
cargo test --manifest-path=src-tauri/Cargo.toml
cargo clippy --manifest-path=src-tauri/Cargo.toml
```

Then run the app and verify IPC messages pass validation:
```bash
npm run tauri:dev:trace
```
Check logs for any "topic not allowed" warnings.
