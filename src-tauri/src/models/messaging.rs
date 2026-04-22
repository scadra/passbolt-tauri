use std::collections::HashSet;

use crate::topics::*;

/// Aggregates all known topics into a whitelist for IPC validation.
pub struct AllowedTopics {
    topics: HashSet<String>,
}

impl AllowedTopics {
    pub fn new() -> Self {
        let mut topics = HashSet::new();

        // Collect all topic values from every topic module
        let all_topic_sets: Vec<Vec<&str>> = vec![
            account_topics::AccountTopics::values(),
            account_recovery_topics::AccountRecoveryTopics::values(),
            action_logs_topics::ActionLogsTopics::values(),
            auth_import_topics::AuthImportTopics::values(),
            authentication_topics::AuthenticationTopics::values(),
            clipboard_topics::ClipboardTopics::values(),
            comment_topics::CommentTopics::values(),
            favorite_topics::FavoriteTopics::values(),
            folder_topics::FolderTopics::values(),
            group_topics::GroupTopics::values(),
            import_export_topics::ImportExportTopics::values(),
            keyring_topics::KeyringTopics::values(),
            locale_storage_topics::LocalStorageTopics::values(),
            locale_topics::LocaleTopics::values(),
            metadata_topics::MetadataTopics::values(),
            mfa_topics::MfaTopics::values(),
            password_expiry_topics::PasswordExpiryTopics::values(),
            password_generator_topics::PasswordGeneratorTopics::values(),
            password_policies_topics::PasswordPoliciesTopics::values(),
            permission_topics::PermissionTopics::values(),
            powned_password_topics::PownedPasswordTopics::values(),
            progress_topics::ProgressTopics::values(),
            rbac_topics::RbacTopics::values(),
            resource_topics::ResourceTopics::values(),
            resource_type_topics::ResourceTypeTopics::values(),
            role_topics::RoleTopics::values(),
            secret_history_topics::SecretHistoryTopics::values(),
            secret_topics::SecretTopics::values(),
            settings_topics::SettingTopics::values(),
            share_topics::ShareTopics::values(),
            tag_topics::TagTopics::values(),
            theme_topics::ThemeTopics::values(),
            user_topics::UserTopics::values(),
        ];

        for set in all_topic_sets {
            for topic in set {
                topics.insert(topic.to_string());
            }
        }

        // Add native-handled topics
        let native = vec![
            crate::config::BACKGROUND_READY,
            crate::config::RENDERED_READY,
            crate::config::BACKGROUND_LISTENER_READY,
            crate::config::BACKGROUND_STORE_PASSPHRASE,
            crate::config::BACKGROUND_AUTHENTICATION_ERROR,
            crate::config::BACKGROUND_SET_THEME,
            crate::config::BACKGROUND_SET_LOCALE,
            crate::config::BACKGROUND_SET_SECURITY_TOKEN,
            crate::config::BACKGROUND_ROTATE_KEY,
            crate::config::BACKGROUND_DOWNLOAD_FILE,
            crate::config::BACKGROUND_GET_COOKIE,
            crate::config::BACKGROUND_CLIPBOARD,
            crate::config::ACCOUNT_SET_CURRENT,
            crate::config::SAVE_ACCOUNT,
            crate::config::BACKGROUND_HTTP_FETCH,
            crate::config::ERROR,
            crate::config::BACKGROUND_IMPORT,
            crate::config::BACKGROUND_AFTER_LOGOUT,
            crate::config::RENDERED_RELOAD,
            crate::config::BACKGROUND_CLIPBOARD_SET_TEXT,
            crate::config::RENDERED_ASSET_FETCH,
        ];
        for topic in native {
            topics.insert(topic.to_string());
        }

        Self { topics }
    }

    /// Returns true if the topic is known (registered or a pending requestId).
    pub fn is_allowed(&self, topic: &str) -> bool {
        self.topics.contains(topic)
    }
}
