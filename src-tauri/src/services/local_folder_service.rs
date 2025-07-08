use std::path::PathBuf;
use std::fs;

pub struct LocalFolderService {
    root_path: PathBuf,
    common_csp: String,
}

impl LocalFolderService {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root_path = if current_dir.to_string_lossy().contains("src-tauri") {
            current_dir.parent().unwrap_or(&current_dir).to_path_buf()
        } else {
            current_dir.clone()
        };
        
        println!("🗂️ LocalFolderService root path: {}", root_path.display());
        
        Self { 
            root_path,
            common_csp: "default-src 'none'; form-action 'none';".to_string(),
        }
    }

    pub fn serve_file(&self, path: &str) -> Result<tauri::http::Response<Vec<u8>>, Box<dyn std::error::Error>> {
        let clean_path = path.strip_prefix('/').unwrap_or(path);
        let full_path = self.root_path.join(clean_path);

        match fs::read(&full_path) {
            Ok(content) => {
                let content_type = self.get_content_type(path);
                Ok(tauri::http::Response::builder()
                    .status(200)
                    .header("Content-Type", content_type)
                    .body(content)?)
            },
            Err(_) => {
                Ok(tauri::http::Response::builder()
                    .status(404)
                    .body(format!("File not found: {}", path).into_bytes())?)
            }
        }
    }

    /// Create index file for rendered webview
    pub fn create_rendered_index(&self, script: &str, stylesheet: &str, trusted_domain: Option<&str>, uuid: &str) -> Result<tauri::http::Response<Vec<u8>>, Box<dyn std::error::Error>> {
        let csp = self.get_rendered_csp(script, trusted_domain, uuid);
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
    <script>window.INSTANCE_UUID = '{}';</script>
    <script src="passbolt://{}.passbolt.local/apps/rendered-webview/dist/{}.js"></script>
</body>
</html>"#, 
            csp, uuid, stylesheet, uuid, uuid, uuid, script
        );

        Ok(tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("X-Frame-Options", "DENY")
            .header("X-Instance-UUID", uuid)
            .body(content.into_bytes())?)
    }

    /// Create index file for background webview  
    pub fn create_background_index(&self, script: &str, trusted_domain: Option<&str>, uuid: &str) -> Result<tauri::http::Response<Vec<u8>>, Box<dyn std::error::Error>> {
        let csp = self.get_background_csp(script, trusted_domain, uuid);
        let content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    {}
    <meta name="instance-uuid" content="{}">
</head>
<body>
    <script>window.INSTANCE_UUID = '{}';</script>
    <script src="passbolt://{}.passbolt.local/apps/background-webview/dist/{}.js"></script>
</body>
</html>"#, 
            csp, uuid, uuid, uuid, script
        );

        Ok(tauri::http::Response::builder()
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("X-Frame-Options", "DENY")
            .header("X-Instance-UUID", uuid)
            .body(content.into_bytes())?)
    }

    /// Get the rendered CSP content
    fn get_rendered_csp(&self, script: &str, trusted_domain: Option<&str>, uuid: &str) -> String {
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

        format!(r#"<meta http-equiv="Content-Security-Policy" content="{} script-src 'unsafe-inline' passbolt://{}.passbolt.local/apps/rendered-webview/; style-src 'self' passbolt://{}.local/apps/rendered-webview/dist/themes/; font-src passbolt://{}.local/apps/rendered-webview/fonts/; {}" />"#, 
            self.common_csp, uuid, uuid, uuid, csp)
    }

    /// Get the background CSP content
    fn get_background_csp(&self, script: &str, trusted_domain: Option<&str>, uuid: &str) -> String {
        let csp_allowed_url = self.get_csp_allowed_url_on_trusted_domain(trusted_domain);
        let connect_src = format!("connect-src passbolt://{}.passbolt.local/apps/background-webview/dist/locales/", uuid);

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

        format!(r#"<meta http-equiv="Content-Security-Policy" content="{} script-src 'unsafe-inline' passbolt://{}.passbolt.local/apps/background-webview/; {}" />"#, 
            self.common_csp, uuid, csp)
    }

    /// Add a slash in case the trusted domain does not contain it for CSP
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
        match path.split('.').last() {
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