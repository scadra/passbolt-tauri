use std::collections::HashSet;
use tauri::{AppHandle, Emitter};
use url::Url;

use crate::topics::{
    account_recovery_topics::AccountRecoveryTopics,
    account_topics::AccountTopics,
    action_logs_topics::ActionLogsTopics,
    auth_import_topics::AuthImportTopics,
    authentication_topics::AuthenticationTopics,
    clipboard_topics::ClipboardTopics,
    comment_topics::CommentTopics,
    favorite_topics::FavoriteTopics,
    folder_topics::FolderTopics,
    group_topics::GroupTopics,
    import_export_topics::ImportExportTopics,
    keyring_topics::KeyringTopics,
    locale_storage_topics::LocalStorageTopics,
    locale_topics::LocaleTopics,
    metadata_topics::MetadataTopics,
    mfa_topics::MfaTopics,
    password_expiry_topics::PasswordExpiryTopics,
    password_generator_topics::PasswordGeneratorTopics,
    password_policies_topics::PasswordPoliciesTopics,
    permission_topics::PermissionTopics,
    powned_password_topics::PownedPasswordTopics,
    progress_topics::ProgressTopics,
    rbac_topics::RbacTopics,
    resource_topics::ResourceTopics,
    resource_type_topics::ResourceTypeTopics,
    role_topics::RoleTopics,
    secret_history_topics::SecretHistoryTopics,
    secret_topics::SecretTopics,
    settings_topics::SettingTopics,
    share_topics::ShareTopics,
    tag_topics::TagTopics,
    theme_topics::ThemeTopics,
    user_topics::UserTopics,
};

use crate::AppResult;
use crate::config;
use crate::error::AppError;
use crate::state::AppState;
use crate::models::messaging::{AllowedTopics, IPC};

pub struct RenderedTopicService {
    app_handle: AppHandle,
    topics: HashSet<String>,
}

impl RenderedTopicService {
    pub fn new(app_handle: AppHandle) -> Self {
        let mut topics = HashSet::new();

        for group in &[
            AccountTopics::all(),
            AccountRecoveryTopics::all(),
            ActionLogsTopics::all(),
            AuthImportTopics::all(),
            AuthenticationTopics::all(),
            ClipboardTopics::all(),
            CommentTopics::all(),
            FavoriteTopics::all(),
            FolderTopics::all(),
            GroupTopics::all(),
            ImportExportTopics::all(),
            KeyringTopics::all(),
            LocaleTopics::all(),
            LocalStorageTopics::all(),
            MetadataTopics::all(),
            MfaTopics::all(),
            PasswordExpiryTopics::all(),
            PasswordGeneratorTopics::all(),
            PasswordPoliciesTopics::all(),
            PermissionTopics::all(),
            PownedPasswordTopics::all(),
            ProgressTopics::all(),
            RbacTopics::all(),
            ResourceTopics::all(),
            ResourceTypeTopics::all(),
            RoleTopics::all(),
            SecretHistoryTopics::all(),
            SecretTopics::all(),
            SettingTopics::all(),
            ShareTopics::all(),
            TagTopics::all(),
            ThemeTopics::all(),
            UserTopics::all(),
        ] {
            for &topic in *group {
                topics.insert(topic.to_string());
            }
        }

        for &topic in AllowedTopics::all() {
            topics.insert(topic.to_string());
        }

        Self { app_handle, topics }
    }

    pub fn is_topic_allowed(&self, topic: &str) -> bool {
        self.topics.contains(topic)
    }

    pub fn proceed_message(&mut self, ipc: IPC, state: &AppState) -> AppResult<()> {
        log::debug!("[Rendered → Background] topic={} request_id={}", ipc.topic, ipc.request_id);
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("[Rendered → Background] message={}", serde_json::to_string(&ipc).unwrap_or_default());
        }

        if !self.topics.contains(&ipc.topic) && !state.proceed_request_id(&ipc.topic) {
            log::warn!("[Rendered → Background] REJECTED topic={}", ipc.topic);
            return Err(AppError::UnauthorizedTopic {
                origin: "Rendered webview".to_string(),
                topic: ipc.topic.clone(),
            });
        } else if self.can_open_to_browser(&ipc) {
            if let Some(url_str) = ipc.message.as_ref().and_then(|v| v.as_str()) {
                let _url = Url::parse(url_str)?;
                // TODO: implement launch_uri_async
            }
        } else if self.topics.contains(&ipc.topic) {
            state.add_request_id(&ipc.request_id);
        }

        if ipc.topic == RbacTopics::FIND_ME {
            state.add_pending_request(&ipc.request_id, RbacTopics::FIND_ME);
        }

        if ipc.topic == AllowedTopics::RENDERED_RELOAD {
            // TODO: implement reload (reload both webviews)
            return Ok(());
        }

        self.post_to_background(&ipc)?;

        Ok(())
    }

    fn post_to_background(&self, ipc: &IPC) -> AppResult<()> {
        self.app_handle.emit_to(config::BACKGROUND_WEBVIEW_LABEL, config::BACKGROUND_IPC_FORWARD, ipc)?;
        Ok(())
    }

    pub fn can_open_to_browser(&self, ipc: &IPC) -> bool {
        if let Some(url_str) = ipc.message.as_ref().and_then(|v| v.as_str()) {
            url_str.starts_with(config::HELP_URL_PREFIX)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the topic whitelist without requiring AppHandle
    fn build_topic_set() -> HashSet<String> {
        let mut topics = HashSet::new();
        for group in &[
            AccountTopics::all(),
            AccountRecoveryTopics::all(),
            ActionLogsTopics::all(),
            AuthImportTopics::all(),
            AuthenticationTopics::all(),
            ClipboardTopics::all(),
            CommentTopics::all(),
            FavoriteTopics::all(),
            FolderTopics::all(),
            GroupTopics::all(),
            ImportExportTopics::all(),
            KeyringTopics::all(),
            LocaleTopics::all(),
            LocalStorageTopics::all(),
            MetadataTopics::all(),
            MfaTopics::all(),
            PasswordExpiryTopics::all(),
            PasswordGeneratorTopics::all(),
            PasswordPoliciesTopics::all(),
            PermissionTopics::all(),
            PownedPasswordTopics::all(),
            ProgressTopics::all(),
            RbacTopics::all(),
            ResourceTopics::all(),
            ResourceTypeTopics::all(),
            RoleTopics::all(),
            SecretHistoryTopics::all(),
            SecretTopics::all(),
            SettingTopics::all(),
            ShareTopics::all(),
            TagTopics::all(),
            ThemeTopics::all(),
            UserTopics::all(),
        ] {
            for &topic in *group {
                topics.insert(topic.to_string());
            }
        }
        for &topic in AllowedTopics::all() {
            topics.insert(topic.to_string());
        }
        topics
    }

    #[test]
    fn test_whitelist_contains_expected_topics() {
        let topics = build_topic_set();
        assert!(topics.contains("passbolt.auth.login"));
        assert!(topics.contains("passbolt.auth.logout"));
        assert!(topics.contains("passbolt.resources.create"));
        assert!(topics.contains("passbolt.resources.update"));
        assert!(topics.contains("passbolt.rendered.is-ready"));
        assert!(topics.contains("passbolt.tab.reload"));
    }

    #[test]
    fn test_whitelist_rejects_unknown_topics() {
        let topics = build_topic_set();
        assert!(!topics.contains("passbolt.admin.evil"));
        assert!(!topics.contains(""));
        assert!(!topics.contains("unknown.topic"));
    }

    #[test]
    fn test_can_open_to_browser() {
        let ipc = IPC {
            topic: "test".to_string(),
            message: Some(serde_json::json!("https://help.passbolt.com/faq")),
            request_id: "123".to_string(),
            status: None,
            with_multiple_param: None,
        };
        assert!(ipc.message.as_ref().and_then(|v| v.as_str())
            .map_or(false, |url| url.starts_with(config::HELP_URL_PREFIX)));

        let ipc_evil = IPC {
            topic: "test".to_string(),
            message: Some(serde_json::json!("https://evil.com")),
            request_id: "123".to_string(),
            status: None,
            with_multiple_param: None,
        };
        assert!(!ipc_evil.message.as_ref().and_then(|v| v.as_str())
            .map_or(false, |url| url.starts_with(config::HELP_URL_PREFIX)));
    }

    #[test]
    fn test_can_open_to_browser_no_message() {
        let ipc = IPC {
            topic: "test".to_string(),
            message: None,
            request_id: "123".to_string(),
            status: None,
            with_multiple_param: None,
        };
        assert!(!ipc.message.as_ref().and_then(|v| v.as_str())
            .map_or(false, |url| url.starts_with(config::HELP_URL_PREFIX)));
    }
}
