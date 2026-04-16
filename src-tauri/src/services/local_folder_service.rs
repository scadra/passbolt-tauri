use std::path::PathBuf;
use std::fs;
use log::info;

use crate::AppResult;
use crate::error::AppError;
use crate::models::asset_model::{RenderedAssets, BackgroundAssets, IsolationAssets};

pub struct LocalFolderService {
    common_csp: String,
}

impl Default for LocalFolderService {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalFolderService {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root_path = if current_dir.to_string_lossy().contains("src-tauri") {
            current_dir.parent().unwrap_or(&current_dir).to_path_buf()
        } else {
            current_dir.clone()
        };

        info!("LocalFolderService root path: {}", root_path.display());

        Self {
            common_csp: "default-src 'none'; form-action 'none';".to_string(),
        }
    }

    pub fn generate_isolation_html(&self) -> AppResult<()> {
        info!("Generating isolation HTML...");

        let _security_js = Self::get_security_js();

        let html_content = r#"<!DOCTYPE html>
                <html>
                <head>
                    <meta charset="UTF-8">
                    <title>Passbolt Isolation</title>
                </head>
                <body>
                    <script src="index.js" ></script>
                </body>
                </html>"#.to_string();

        fs::write("../src/isolation/index.html", html_content)?;

        info!("Isolation HTML generated");
        Ok(())
    }

    fn get_security_js() -> String {
        match IsolationAssets::get("index.js") {
            Some(file) => {
                info!("Using embedded isolation index.js");
                String::from_utf8_lossy(&file.data).to_string()
            },
            None => String::new()
        }
    }

    pub fn serve_file(&self, path: &str) -> AppResult<tauri::http::Response<Vec<u8>>> {
        let clean_path = path.strip_prefix('/').unwrap_or(path);

        if let Some(response) = self.serve_embedded_asset(clean_path)? {
            return Ok(response);
        }

        log::warn!("File not found in embedded assets: {}", clean_path);
        Ok(tauri::http::Response::builder()
            .status(404)
            .header("X-Security-Mode", "embedded-only")
            .body(format!("Embedded file not found: {}", path).into_bytes())
            .map_err(|e| AppError::Webview(e.to_string()))?)
    }

    fn serve_embedded_asset(&self, path: &str) -> AppResult<Option<tauri::http::Response<Vec<u8>>>> {
        let asset_data = if path.starts_with("src/isolation-rendered/") {
            let asset_path = path.strip_prefix("src/isolation-rendered/").unwrap_or(path);
            IsolationAssets::get(asset_path).map(|file| {
                info!("Serving rendered isolation asset: {}", asset_path);
                (file.data.to_vec(), "embedded-isolation-rendered")
            })
        } else if path.starts_with("src/isolation-background/") {
            let asset_path = path.strip_prefix("src/isolation-background/").unwrap_or(path);
            IsolationAssets::get(asset_path).map(|file| {
                info!("Serving background isolation asset: {}", asset_path);
                (file.data.to_vec(), "embedded-isolation-background")
            })
        } else if path.starts_with("apps/rendered-webview/") {
            let asset_path = path.strip_prefix("apps/rendered-webview/").unwrap_or(path);
            RenderedAssets::get(asset_path).map(|file| {
                info!("Serving rendered asset: {}", asset_path);
                (file.data.to_vec(), "embedded-rendered")
            })
        } else if path.starts_with("apps/background-webview/") {
            let asset_path = path.strip_prefix("apps/background-webview/").unwrap_or(path);
            BackgroundAssets::get(asset_path).map(|file| {
                info!("Serving background asset: {}", asset_path);
                (file.data.to_vec(), "embedded-background")
            })
        } else {
            None
        };

        if let Some((content, source)) = asset_data {
            let content_type = self.get_content_type(path);
            return Ok(Some(tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", content_type)
                .header("X-Content-Source", source)
                .header("Cache-Control", "public, max-age=31536000, immutable")
                .header("X-Security-Level", "embedded")
                .body(content)
                .map_err(|e| AppError::Webview(e.to_string()))?));
        }

        Ok(None)
    }

    pub fn create_rendered_index(&self, script: &str, stylesheet: &str, trusted_domain: Option<&str>, isolation_url: Option<&str>, uuid: &str) -> AppResult<tauri::http::Response<Vec<u8>>> {
        let csp = self.get_rendered_csp(script, trusted_domain, isolation_url, uuid);
        let content = format!(
            r#"<!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                {}
                <meta name="instance-uuid" content="{}">
                <script id="stylesheet-manager" data-file="{}" src="passbolt://{}.passbolt.local/apps/rendered-webview/src/js/stylesheet.js"></script>
            </head>
            <body>
`                <script src="passbolt://{}.passbolt.local/apps/rendered-webview/dist/{}.js"></script>
            </body>
            </html>"#,
            csp, uuid, stylesheet, uuid, uuid, script
        );

        Ok(tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("X-Frame-Options", "DENY")
            .header("X-Instance-UUID", uuid)
            .body(content.into_bytes())
            .map_err(|e| AppError::Webview(e.to_string()))?)
    }

    pub fn create_background_index(&self, script: &str, trusted_domain: Option<&str>, isolation_url: Option<&str>, uuid: &str) -> AppResult<tauri::http::Response<Vec<u8>>> {
        let csp = self.get_background_csp(script, trusted_domain, isolation_url, uuid);
        let content = format!(
            r#"<!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                {}
                <meta name="instance-uuid" content="{}">
            </head>
            <body>
                <script src="passbolt://{}.passbolt.local/apps/background-webview/dist/{}.js"></script>
            </body>
            </html>"#,
            csp, uuid, uuid, script
        );

        Ok(tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("X-Frame-Options", "DENY")
            .header("X-Instance-UUID", uuid)
            .body(content.into_bytes())
            .map_err(|e| AppError::Webview(e.to_string()))?)
    }

    fn get_rendered_csp(&self, script: &str, trusted_domain: Option<&str>, isolation_url: Option<&str>, uuid: &str) -> String {
        let csp_allowed_url = self.get_csp_allowed_url_on_trusted_domain(trusted_domain);
        let img_src = format!("img-src passbolt://{}.passbolt.local/apps/rendered-webview/img/", uuid);

        let csp = match script {
            "rendered-auth" => {
                format!("{}; connect-src passbolt://{}.passbolt.local/apps/rendered-webview/dist/locales/;",
                    if let Some(url) = &csp_allowed_url { format!("{} {}", img_src, url) } else { img_src.to_string() }, uuid)
            },
            "rendered-workspace" => {
                format!("{} data: blob:; connect-src passbolt://{}.passbolt.local/apps/rendered-webview/dist/locales/;",
                    if let Some(url) = &csp_allowed_url { format!("{} {}", img_src, url) } else { img_src.to_string() }, uuid)
            },
            _ => format!("{};", img_src)
        };

        format!(r#"<meta http-equiv="Content-Security-Policy" content="{} script-src passbolt://{}.passbolt.local/apps/rendered-webview/; style-src 'self' passbolt://{}.passbolt.local/apps/rendered-webview/dist/themes/; font-src passbolt://{}.passbolt.local/apps/rendered-webview/fonts/; connect-src ipc://localhost/plugin:event|listen ipc://localhost/plugin:event|emit; frame-src {}://localhost ;{}" />"#,
            self.common_csp, uuid, uuid, uuid, isolation_url.unwrap_or(""), csp)
    }

    fn get_background_csp(&self, script: &str, trusted_domain: Option<&str>, isolation_url: Option<&str>, uuid: &str) -> String {
        let csp_allowed_url = self.get_csp_allowed_url_on_trusted_domain(trusted_domain);
        let connect_src = format!("connect-src ipc://localhost/plugin:event|listen ipc://localhost/plugin:event|emit passbolt://{}.passbolt.local/apps/background-webview/dist/locales/", uuid);

        let csp = match script {
            "background-auth" => {
                if let Some(url) = csp_allowed_url {
                    format!("{} {};", connect_src, url)
                } else {
                    format!("{};", connect_src)
                }
            },
            "background-workspace" => {
                if let Some(url) = csp_allowed_url {
                    format!("{} {} https://api.pwnedpasswords.com;", connect_src, url)
                } else {
                    format!("{} https://api.pwnedpasswords.com;", connect_src)
                }
            },
            "background-import" => {
                format!("{};", connect_src)
            },
            _ => format!("{};", connect_src)
        };

        format!(r#"<meta http-equiv="Content-Security-Policy" content="{} script-src passbolt://{}.passbolt.local/apps/background-webview/;frame-src {}://localhost ; {}" />"#,
            self.common_csp, uuid, isolation_url.unwrap_or(""),csp)
    }

    fn get_csp_allowed_url_on_trusted_domain(&self, trusted_domain: Option<&str>) -> Option<String> {
        trusted_domain.map(|domain| {
            if domain.ends_with('/') {
                domain.to_string()
            } else {
                format!("{}/", domain)
            }
        })
    }

    fn get_content_type(&self, path: &str) -> &'static str {
        match path.rsplit('.').next() {
            Some("html") => "text/html; charset=utf-8",
            Some("js") => "application/javascript",
            Some("css") => "text/css",
            Some("json") => "application/json",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("svg") => "image/svg+xml",
            _ => "application/octet-stream",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_detection() {
        let service = LocalFolderService::new();
        assert_eq!(service.get_content_type("file.html"), "text/html; charset=utf-8");
        assert_eq!(service.get_content_type("file.js"), "application/javascript");
        assert_eq!(service.get_content_type("file.css"), "text/css");
        assert_eq!(service.get_content_type("file.json"), "application/json");
        assert_eq!(service.get_content_type("file.png"), "image/png");
        assert_eq!(service.get_content_type("file.jpg"), "image/jpeg");
        assert_eq!(service.get_content_type("file.svg"), "image/svg+xml");
        assert_eq!(service.get_content_type("file.unknown"), "application/octet-stream");
    }

    #[test]
    fn test_csp_allowed_url_trailing_slash() {
        let service = LocalFolderService::new();
        assert_eq!(
            service.get_csp_allowed_url_on_trusted_domain(Some("https://example.com")),
            Some("https://example.com/".to_string())
        );
        assert_eq!(
            service.get_csp_allowed_url_on_trusted_domain(Some("https://example.com/")),
            Some("https://example.com/".to_string())
        );
        assert_eq!(
            service.get_csp_allowed_url_on_trusted_domain(None),
            None
        );
    }
}
