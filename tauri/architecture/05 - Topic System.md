# Topic System

Every IPC message carries a `topic` string. Topics are defined as compile-time constants, aggregated into a whitelist, and validated on every incoming message.

## define_topics! Macro

File: `lib.rs:6-18`

```rust
macro_rules! define_topics {
    ($name:ident { $($const_name:ident => $value:expr),* $(,)? }) => {
        pub struct $name;
        impl $name {
            $(pub const $const_name: &'static str = $value;)*
            pub fn values() -> Vec<&'static str> {
                vec![$($value),*]
            }
        }
    };
}
```

Usage example (`topics/authentication_topics.rs`):
```rust
define_topics!(AuthenticationTopics {
    VERIFY_PASSPHRASE    => "passbolt.auth.verify-passphrase",
    DESKTOP_AUTHENTICATE => "passbolt.auth.login",
    AFTER_LOGIN          => "passbolt.auth.after-login",
    LOG_OUT              => "passbolt.auth.logout",
    REQUIRE_MFA          => "passbolt.auth.redirect-to-mfa",
});
```

Generates a struct `AuthenticationTopics` with `const` string literals and `values()` returning them all.

## AllowedTopics Whitelist

File: `models/messaging.rs`

```rust
pub struct AllowedTopics {
    topics: HashSet<String>,
}

impl AllowedTopics {
    pub fn new() -> Self {
        let mut topics = HashSet::new();
        // Collects values() from all 33 topic modules
        topics.extend(AuthenticationTopics::values().into_iter().map(String::from));
        topics.extend(ResourceTopics::values().into_iter().map(String::from));
        // ... all 33 modules ...
        // Plus native topics from config.rs
        Self { topics }
    }

    pub fn is_allowed(&self, topic: &str) -> bool {
        self.topics.contains(topic)
    }
}
```

Built once at startup by `RenderedTopicService::new()`.

## The 33 Modules

File: `topics/mod.rs`

| Module | Topic prefix | Examples |
|--------|-------------|----------|
| `authentication_topics` | `passbolt.auth.*` | login, after-login, logout, redirect-to-mfa |
| `resource_topics` | `passbolt.resources.*` | find-all, create, update, delete |
| `secret_topics` | `passbolt.secret.*` | passphrase-request, decrypt |
| `folder_topics` | `passbolt.folders.*` | create, update, move-strategy-request |
| `user_topics` | `passbolt.users.*` | find-all, find-logged-in-user |
| `group_topics` | `passbolt.groups.*` | create, update, delete |
| `share_topics` | `passbolt.share.*` | search-aros, resources |
| `permission_topics` | `passbolt.permissions.*` | find-all |
| `tag_topics` | `passbolt.tags.*` | find-all, update |
| `comment_topics` | `passbolt.comments.*` | find-all, create |
| `favorite_topics` | `passbolt.favorites.*` | add, delete |
| `clipboard_topics` | `passbolt.clipboard.*` | copy |
| `locale_storage_topics` | `passbolt.background/rendered.localstorage-*` | update, delete, clear |
| `locale_topics` | `passbolt.locale.*` | get |
| `theme_topics` | `passbolt.theme.*` | find-all |
| `settings_topics` | `passbolt.site-settings.*` | get |
| `rbac_topics` | `passbolt.rbacs.*` | find-me |
| `progress_topics` | `passbolt.progress.*` | open-dialog, update, close-dialog |
| `mfa_topics` | `passbolt.mfa-policy.*` | get |
| `metadata_topics` | `passbolt.metadata.*` | find-all-types |
| `password_generator_topics` | `passbolt.password-generator.*` | settings |
| `password_policies_topics` | `passbolt.password-policies.*` | get |
| `password_expiry_topics` | `passbolt.password-expiry.*` | settings |
| `powned_password_topics` | `passbolt.powned-password.*` | check |
| `import_export_topics` | `passbolt.import/export.*` | resources |
| `keyring_topics` | `passbolt.keyring.*` | get-public-key |
| `account_topics` | `passbolt.account.*` | set-current |
| `account_recovery_topics` | `passbolt.account-recovery.*` | get-policy |
| `action_logs_topics` | `passbolt.actionlogs.*` | find-all |
| `auth_import_topics` | `passbolt.background.*` | import, save-account |
| `role_topics` | `passbolt.role.*` | get-all |
| `resource_type_topics` | `passbolt.resource-type.*` | find-all |
| `secret_history_topics` | `passbolt.secret-history.*` | find-all |

## Native Topics

Beyond the 33 modules, `config.rs` defines ~20 native topics that Rust handles directly. These are **not** forwarded to the other webview. They are also included in the whitelist.

Listed in `config.rs:18-41`. See [[06 - Routing]] for how they are dispatched.

## RequestId as Topic

In the response flow, the `topic` field of a response is set to the original `requestId`. This is how `BackgroundTopicService` distinguishes responses from new requests: a topic that doesn't match any known constant or the whitelist but matches a registered requestId is treated as a response.
