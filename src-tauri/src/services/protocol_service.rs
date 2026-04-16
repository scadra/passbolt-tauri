use log::{info, warn, error};

use crate::error::AppError;
use crate::config;
use crate::services::local_folder_service::LocalFolderService;
use crate::services::credential_service::CredentialService;

pub struct ProtocolService {
    local_folder_service: LocalFolderService,
    isolation_schema: Option<String>,
}

impl ProtocolService {
    pub fn new(local_folder_service: LocalFolderService, isolation_schema: Option<String>) -> Self {
        Self {
            local_folder_service,
            isolation_schema,
        }
    }

    pub fn handle_request(&self, host: &str, path: &str) -> tauri::http::Response<Vec<u8>> {
        info!("Request: {}://{}{}", config::PROTOCOL_SCHEME, host, path);

        if !host.ends_with(config::DOMAIN_SUFFIX) {
            warn!("Unauthorized domain: {}", host);
            return tauri::http::Response::builder()
                .status(403)
                .body(b"Unauthorized domain".to_vec())
                .unwrap();
        }

        let uuid = host.replace(config::DOMAIN_SUFFIX, "");

        let result = match path {
            "/background" => {
                info!("Serving background app for UUID: {}", uuid);
                self.local_folder_service.create_background_index(
                    "background-import",
                    None,
                    self.isolation_schema.as_deref(),
                    &uuid,
                )
            }
            "/background-auth" => {
                info!("Serving background-auth app for UUID: {}", uuid);
                let trusted_domain = CredentialService::get_account_metadata()
                    .ok()
                    .map(|m| m.domain);
                self.local_folder_service.create_background_index(
                    "background-auth",
                    trusted_domain.as_deref(),
                    self.isolation_schema.as_deref(),
                    &uuid,
                )
            }
            "/app" => {
                info!("Serving rendered app for UUID: {}", uuid);
                self.local_folder_service.create_rendered_index(
                    "rendered-import",
                    "ext_authentication.min.css",
                    None,
                    self.isolation_schema.as_deref(),
                    &uuid,
                )
            }
            "/app-auth" => {
                info!("Serving rendered auth app for UUID: {}", uuid);
                let trusted_domain = CredentialService::get_account_metadata()
                    .ok()
                    .map(|m| m.domain);
                self.local_folder_service.create_rendered_index(
                    "rendered-auth",
                    "ext_authentication.min.css",
                    trusted_domain.as_deref(),
                    self.isolation_schema.as_deref(),
                    &uuid,
                )
            }
            path if path.starts_with("/apps/") => {
                info!("Serving webview asset: {}", path);
                self.local_folder_service.serve_file(path)
            }
            _ => {
                warn!("Not found: {}", path);
                tauri::http::Response::builder()
                    .status(404)
                    .body(b"Page not found".to_vec())
                    .map_err(|e| AppError::Webview(e.to_string()))
            }
        };

        result.unwrap_or_else(|e| {
            error!("Error serving content: {}", e);
            tauri::http::Response::builder()
                .status(500)
                .body(b"Internal server error".to_vec())
                .unwrap()
        })
    }
}
